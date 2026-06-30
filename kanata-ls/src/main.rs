use lsp_server::{Connection, ErrorCode, Message, Response};
use lsp_types::{
    notification::PublishDiagnostics,
    request::{Formatting, GotoDefinition, HoverRequest, PrepareRenameRequest, Rename, Request},
    InitializeParams,
};

use kanata_ls::KanataLanguageServer;

fn main() -> anyhow::Result<()> {
    println!("kanata-lsp starting");

    let (connection, io_threads) = Connection::stdio();

    let (id, params) = connection.initialize_start()?;
    let params: InitializeParams = serde_json::from_value(params)?;

    let mut kls = KanataLanguageServer::new(params.clone());
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
                for diag_params in kls.take_pending_diagnostics() {
                    let notif = lsp_server::Notification::new(
                        <PublishDiagnostics as lsp_types::notification::Notification>::METHOD
                            .to_string(),
                        diag_params,
                    );
                    connection.sender.send(Message::Notification(notif))?;
                }
            }
            Message::Response(_) => {}
        }
    }

    drop(connection);
    io_threads.join()?;
    println!("kanata-lsp stopped");
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
