use lsp_server::{Connection, ErrorCode, Message, Response};
use lsp_types::{
    notification::PublishDiagnostics,
    request::{Formatting, GotoDefinition, HoverRequest, PrepareRenameRequest, Rename, Request},
    InitializeParams, PublishDiagnosticsParams,
};

use kanata_ls::{log, KanataLanguageServer};

pub fn main() -> anyhow::Result<()> {
    eprintln!("kanata-ls starting");

    let (connection, _io_threads) = Connection::stdio();
    let connection: &'static _ = Box::leak(Box::new(connection));

    log!("waiting for client initialize");
    let (id, params) = connection.initialize_start()?;

    log!("parsing the received message as InitializeParams struct");
    let params: InitializeParams = serde_json::from_value(params)?;

    let send_diagnostics_callback: &'static _ =
        Box::leak(Box::new(|diag_params: &PublishDiagnosticsParams| {
            let notif = lsp_server::Notification::new(
                <PublishDiagnostics as lsp_types::notification::Notification>::METHOD.to_string(),
                diag_params,
            );
            connection.sender.send(Message::Notification(notif))?;
            Ok(())
        }));

    let mut kls = KanataLanguageServer::new(params.clone(), send_diagnostics_callback);
    let init_result = kls.initialize(&params);

    connection.initialize_finish(id, serde_json::to_value(init_result)?)?;

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    break;
                }
                let resp = dispatch_request(&mut kls, req);
                connection.sender.send(Message::Response(resp))?;
            }
            Message::Notification(not) => {
                kls.on_notification(&not.method, not.params);
            }
            Message::Response(_) => {}
        }
    }

    // No cleanup becase there's a mess with the static lifetimes.
    // It's not a problem because it's the end of the program anyway.

    // drop(connection);
    // io_threads.join()?;

    Ok(())
}

fn dispatch_request(kls: &mut KanataLanguageServer, req: lsp_server::Request) -> Response {
    let id = req.id.clone();
    match req.method.as_str() {
        Formatting::METHOD => {
            let params = serde_json::from_value(req.params).unwrap();
            let result = kls.on_document_formatting(&params);
            Response::new_ok(id, serde_json::to_value(result).unwrap())
        }
        GotoDefinition::METHOD => {
            let params = serde_json::from_value(req.params).unwrap();
            let result = kls.on_go_to_definition(&params);
            Response::new_ok(id, serde_json::to_value(result).unwrap())
        }
        HoverRequest::METHOD => {
            let params = serde_json::from_value(req.params).unwrap();
            let result = kls.on_hover(&params);
            Response::new_ok(id, serde_json::to_value(result).unwrap())
        }
        PrepareRenameRequest::METHOD => {
            let params = serde_json::from_value(req.params).unwrap();
            let result = kls.on_prepare_rename(&params);
            Response::new_ok(id, serde_json::to_value(result).unwrap())
        }
        Rename::METHOD => {
            let params = serde_json::from_value(req.params).unwrap();
            let result = kls.on_rename(&params);
            Response::new_ok(id, serde_json::to_value(result).unwrap())
        }
        method => Response::new_err(
            id,
            ErrorCode::MethodNotFound as i32,
            format!("unknown request method: {method}"),
        ),
    }
}
