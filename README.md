## Small example of using WasmEdge sdk to define a Rust host library

Includes two functions which access the Memory of the wasm application, manipulate it, and exit the functions.


Build `host_library` and `wasm_app` in separate terminals  

To run   
```
WASMEDGE_PLUGIN_PATH=/home/charles/we/yolo_ffmpeg_plugin/   wasmedge  ./target/wasm32-wasi/release/wasm_app.wasm
```  

