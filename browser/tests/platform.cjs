// Actual JavaScript journal + real Rust WASM, including I/O fault injection.
import('./resume.mjs').catch(error=>{console.error(error);process.exitCode=1;});
