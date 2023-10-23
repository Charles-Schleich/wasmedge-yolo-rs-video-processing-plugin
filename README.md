## WasmEdge Yolo-rs Video Processing plugin 

This is a small plugin developed to support video processing of [yolo-rs](https://github.com/Charles-Schleich/yolo-rs)
Using FFMPEG for video processing. 

Build `host_library` and `wasm_app` in separate terminals

#### To build:
Terminal 1:  
`cd host_library && cargo build --release`  
Terminal 2:  
`cd wasm_app && cargo build --release`  

#### To run:
From project root  
`WASMEDGE_PLUGIN_PATH=/home/charles/we/yolo_ffmpeg_plugin/targe/release   wasmedge  ./target/wasm32-wasi/release/wasm_app.wasm` 