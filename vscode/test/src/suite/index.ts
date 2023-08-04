// import { resolve } from 'path';

// import  from 'jest'; // eslint-disable-line node/no-unpublished-import
// import glob from 'glob'; // eslint-disable-line node/no-unpublished-import

// type Callback = (error: Error | null, failures?: number) => void;

// export function run(cwd: string, cb: Callback): void {
//   const mocha = new Mocha({ ui: 'tdd', color: true, timeout: 1000 * 60 });

//   glob.globSync('**/**.test.js', { cwd }).forEach(f => {
//     // if (err) return cb(err);

//     // Add files to the test suite
//     mocha.addFile(resolve(cwd, f));
//   });

//   try {
//     mocha.run(failures => cb(null, failures));
//   } catch (err) {
//     console.error(err);
//     cb(err as Error);
//   }
// }
