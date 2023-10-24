use std::{
    io::{Read, Write},
    mem::size_of,
    sync::{Arc, Mutex},
};

mod dump_frames;

use ffmpeg::frame;

use image::GenericImage;
use wasmedge_sdk::{
    error::HostFuncError,
    host_function,
    plugin::{ffi, PluginDescriptor, PluginModuleBuilder, PluginVersion},
    Caller, NeverType, WasmValue,
};

use log::debug;

#[host_function]
fn proc_vec(_caller: Caller, args: Vec<WasmValue>) -> Result<Vec<WasmValue>, HostFuncError> {
    println!("Proc Vec");
    let mut main_memory = _caller.memory(0).unwrap();
    let buf_ptr = args[0].to_i32() as u32;
    let buf_len = args[1].to_i32() as usize;
    let buf_capacity = args[2].to_i32() as usize;
    let pointer = main_memory
        .data_pointer_mut(buf_ptr as u32, buf_len as u32)
        .expect("Could not get Data pointer");

    let mut vec = unsafe { Vec::from_raw_parts(pointer, buf_len, buf_capacity) };

    println!("Lib Vec {:?}", vec);
    vec[0] = 100;
    // vec[9] = 100;
    println!("vec {:?}", vec);
    println!("vec {:?}", vec);
    std::mem::forget(vec); // Need to forget x otherwise we get a double free
    let value_y = 0;
    let ret_y = WasmValue::from_i32(value_y);
    Ok(vec![ret_y])
}

#[host_function]
fn proc_string(_caller: Caller, args: Vec<WasmValue>) -> Result<Vec<WasmValue>, HostFuncError> {
    println!("Proc String");
    let mut main_memory = _caller.memory(0).unwrap();

    let data_ptr = args[0].to_i32();
    let data_len = args[1].to_i32();
    let data_capacity = args[2].to_i32();

    println!("Main Memory");
    let pointer = main_memory
        .data_pointer_mut(data_ptr as u32, data_len as u32)
        .expect("Could not get Data pointer");
    // println!("Main Memory bytes {:?}", pointer);
    let mut x =
        unsafe { String::from_raw_parts(pointer, data_len as usize, data_capacity as usize) };
    let upper = x.to_uppercase();
    let mut string_ref = unsafe { x.as_bytes_mut() };
    string_ref.write_all(upper.as_bytes()).expect("Failed");
    std::mem::forget(x); // Need to forget x otherwise we get a double free
    Ok(vec![WasmValue::from_i32(0)])
}

#[host_function]
fn load_video(
    caller: Caller,
    args: Vec<WasmValue>,
    data: &mut Arc<Mutex<Frames>>, // data: &mut Frames,
) -> Result<Vec<WasmValue>, HostFuncError> {
    debug!("Load_video");
    let mut data_guard = data.lock().unwrap();
    let mut main_memory = caller.memory(0).unwrap();

    let filename_ptr = args[0].to_i32();
    let filename_len = args[1].to_i32();
    let filaname_capacity = args[2].to_i32();

    let width_ptr = args[3].to_i32() as *mut i32;
    let height_ptr = args[4].to_i32() as *mut i32;

    // TODO: Proper error handling with Expects
    let width_ptr_main_memory = main_memory
        .data_pointer_mut(width_ptr as u32, 1)
        .expect("Could not get Data pointer") as *mut u32;
    let height_ptr_main_memory = main_memory
        .data_pointer_mut(height_ptr as u32, 1)
        .expect("Could not get Data pointer") as *mut u32;
    let filename_ptr_main_memory = main_memory
        .data_pointer_mut(filename_ptr as u32, filename_len as u32)
        .expect("Could not get Data pointer");

    let filename: String = unsafe {
        String::from_raw_parts(
            filename_ptr_main_memory,
            filename_len as usize,
            filaname_capacity as usize,
        )
    };

    debug!("Call FFMPEG dump Frames");
    let res = match dump_frames::dump_frames(&filename) {
        Ok(frames) => {
            if frames.len() > 0 {
                unsafe {
                    *width_ptr_main_memory = frames[0].width();
                    *height_ptr_main_memory = frames[0].height();
                }
            }
            *data_guard = frames;
            Ok(vec![WasmValue::from_i32(data_guard.len() as i32)])
        }
        // TODO: Make Error more clear
        Err(err) => Err(HostFuncError::User(1)),
    };

    println!("Data Len {:?}", data_guard.len());
    std::mem::forget(filename); // Need to forget x otherwise we get a double free
    res
}

#[host_function]
fn get_frame(
    caller: Caller,
    args: Vec<WasmValue>,
    data: &mut Arc<Mutex<VideoFrames>>,
) -> Result<Vec<WasmValue>, HostFuncError> {
    debug!("get_frame");

    let mut main_memory = caller.memory(0).unwrap();
    let idx: i32 = args[0].to_i32();
    let image_buf_ptr = args[1].to_i32();
    let image_buf_len = args[2].to_i32() as usize;
    let image_buf_capacity = args[3].to_i32() as usize;

    let data_guard = data.lock().unwrap();

    debug!("LIB image_buf_ptr {:?}", image_buf_ptr);
    debug!("LIB image_buf_len {:?}", image_buf_len);
    debug!("LIB image_buf_capacity {:?}", image_buf_capacity);
    let image_ptr_wasm_memory = main_memory
        .data_pointer_mut(image_buf_ptr as u32, image_buf_len as u32)
        .expect("Could not get Data pointer");

    let mut vec =
        unsafe { Vec::from_raw_parts(image_ptr_wasm_memory, image_buf_len, image_buf_capacity) };

    if let Some(frame) = data_guard.input_frames.get(idx as usize) {
        debug!("LIB data {:?}", frame.data(0).len());
        println!("copy_from_slice");
        vec.copy_from_slice(frame.data(0));
        println!("copy_from_slice");
    } else {
        // TODO return Error
        todo!("Return error if frame does not exist");
    };

    std::mem::forget(vec); // Need to forget x otherwise we get a double free
    Ok(vec![WasmValue::from_i32(1)])
}

struct VideoFrames {
    input_frames: Frames,
    output_frames: Frames,
}

type Frames = Vec<frame::Video>;

/// Defines Plugin module instance
unsafe extern "C" fn create_test_module(
    _arg1: *const ffi::WasmEdge_ModuleDescriptor,
) -> *mut ffi::WasmEdge_ModuleInstanceContext {
    let module_name = "yolo-video-proc";

    let video_frames = VideoFrames {
        input_frames: Vec::new(),
        output_frames: Vec::new(),
    };

    let video_frames_arc = Box::new(Arc::new(Mutex::new(video_frames)));

    // TODO Wrap i32's in Struct to avoid misuse / mixups
    type ShareFrames = Arc<Mutex<VideoFrames>>;
    type Width = i32;
    type Height = i32;

    let plugin_module = PluginModuleBuilder::<NeverType>::new()
        // .with_func::<(ExternRef, ExternRef), i32, NeverType>("hello", hello, None)
        .with_func::<(i32, i32, i32), i32, NeverType>("proc_vec", proc_vec, None)
        .expect("failed to create host function")
        .with_func::<(i32, i32, i32), i32, NeverType>("proc_string", proc_string, None)
        .expect("failed to create host function")
        .with_func::<(i32, i32, i32, Width, Height), i32, ShareFrames>(
            "load_video",
            load_video,
            Some(video_frames_arc.clone()),
        )
        .expect("failed to create host function")
        .with_func::<(i32, i32, i32, i32), i32, ShareFrames>(
            "get_frame",
            get_frame,
            Some(video_frames_arc.clone()),
        )
        .expect("failed to create host function")
        .build(module_name)
        .expect("failed to create plugin module");

    let boxed_module = Box::new(plugin_module);
    let module = Box::leak(boxed_module);

    module.as_raw_ptr() as *mut _
}

/// Defines PluginDescriptor
#[export_name = "WasmEdge_Plugin_GetDescriptor"]
pub extern "C" fn plugin_hook() -> *const ffi::WasmEdge_PluginDescriptor {
    const NAME: &str = "yolo-video-proc_plugin";
    const DESC: &str = "This is a yolo video processing plugin utilizing FFMPEG";
    let version = PluginVersion::new(0, 0, 0, 0);
    let plugin_descriptor = PluginDescriptor::new(NAME, DESC, version)
        .expect("Failed to create plugin descriptor")
        .add_module_descriptor(NAME, DESC, Some(create_test_module))
        .expect("Failed to add module descriptor");

    let boxed_plugin = Box::new(plugin_descriptor);
    let plugin = Box::leak(boxed_plugin);

    plugin.as_raw_ptr()
}
