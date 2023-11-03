## WasmEdge Yolo-rs Video Processing plugin 

> [!NOTE]  
> This library is a work in progress and is likely to change

This is a small plugin developed for the WasmEdge Runtime to support video processing of [yolo-rs](https://github.com/Charles-Schleich/yolo-rs)
Using FFMPEG for native video processing. 

Build `host_library` and `wasm_app` in separate terminals

#### To build:
Terminal 1:  
`cd host_library && cargo build --release`  
Terminal 2:  
`cd wasm_app && cargo build --release`  

<!-- 
Quick build
cd wasm_app/ && cargo build --release && cd .. && cd host_library/ && cargo build --release && cd .. 
-->

#### To run:
From project root  
`WASMEDGE_PLUGIN_PATH=/home/charles/we/yolo_ffmpeg_plugin/target/release   wasmedge  ./target/wasm32-wasi/release/wasm_app.wasm` 