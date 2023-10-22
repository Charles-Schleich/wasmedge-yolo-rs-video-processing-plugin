use std::{
    io::Write,
    sync::{Arc, Mutex, RwLock},
};

mod dump_frames;

use ffmpeg::frame;
use wasmedge_sdk::{
    error::HostFuncError,
    host_function,
    plugin::{ffi, PluginDescriptor, PluginModuleBuilder, PluginVersion},
    Caller, NeverType, WasmValue,
};

// The host function takes two arguments of WasmValue type:
// the first argument is a reference to MyString
// the second argument is a reference to MyStr
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
    vec[9] = 100;
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
    _caller: Caller,
    args: Vec<WasmValue>,
    data: &mut Arc<Mutex<Frames>>, // data: &mut Frames,
) -> Result<Vec<WasmValue>, HostFuncError> {
    println!("load_video ====================");
    // _caller.instance()
    let mut data_guard = data.lock().unwrap();
    println!("Data Len {:?}", data_guard.len());
    let mut main_memory = _caller.memory(0).unwrap();

    let data_ptr = args[0].to_i32();
    let data_len = args[1].to_i32();
    let data_capacity = args[2].to_i32();

    println!("Main Memory");
    let pointer = main_memory
        .data_pointer_mut(data_ptr as u32, data_len as u32)
        .expect("Could not get Data pointer");

    let filename =
        unsafe { String::from_raw_parts(pointer, data_len as usize, data_capacity as usize) };

    println!("Calling FFMPEG dump Frames");

    let res = match dump_frames::dump_frames(&filename) {
        Ok(frames) => {
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
    _caller: Caller,
    args: Vec<WasmValue>,
    data: &mut Arc<Mutex<Frames>>,
) -> Result<Vec<WasmValue>, HostFuncError> {
    println!("load_video_into_ffmpeg");
    let idx = args[0].to_i32();
    let frame_pointer = args[1].to_i32();

    let mut data_guard = data.lock().unwrap();
    println!("Data Len {:?}", data_guard.len());

    if let Some(frame) = data_guard.get(idx as usize) {
        println!("Frame Exists ! ");
        println!(" {:?} ", frame.data(0));
        println!(" width {:?} ", frame.width());
        println!(" height {:?} ", frame.height());
        // println!(" color_space {:?} ", frame.color_space());
        // println!(" color_primaries {:?} ", frame.color_primaries());
        // println!(" chroma_location {:?} ", frame.chroma_location());
        // println!(" aspect_ratio {:?} ", frame.aspect_ratio());
        // println!(" aspect_ratio {:?} ", frame.aspect_ratio());

        // println!(" color_primaries {:?} ", frame.());
        // println!(" color_primaries {:?} ", frame.color_primaries());
        // println!(" color_primaries {:?} ", frame.color_primaries());

        // save_file();
        let dump_frame = dump_frames::save_file(frame, idx as usize);
        println!("DUMP FRAME OUT{:?}",dump_frame);

        // image::ImageBuffer::from_vec(width, height, buf)
        // let x = image::ImageBuffer::from_raw(frame.width(), frame.height(), frame.data(0));
        // let image = image::load_from_memory(frame.data(0));
        // println!("Load Image from Bytes {:?}", image);
    } else {
        println!("Frame DOES NOT Exist");
    };

    // std::mem::forget(filename); // Need to forget x otherwise we get a double free
    Ok(vec![WasmValue::from_i32(1)])
    // Ok(vec![])
}

type Frames = Vec<frame::Video>;

/// Defines Plugin module instance
unsafe extern "C" fn create_test_module(
    _arg1: *const ffi::WasmEdge_ModuleDescriptor,
) -> *mut ffi::WasmEdge_ModuleInstanceContext {
    let module_name = "yolo-video-proc";

    let frames: Frames = Vec::new();
    let frames_arc = Box::new(Arc::new(Mutex::new(frames)));
    // let frames_arc = Box::new(Arc::new(frames));
    // type ShareFrames = Arc<Frames>;
    type ShareFrames = Arc<Mutex<Frames>>;

    let plugin_module = PluginModuleBuilder::<NeverType>::new()
        // .with_func::<(ExternRef, ExternRef), i32, NeverType>("hello", hello, None)
        .with_func::<(i32, i32, i32), i32, NeverType>("proc_vec", proc_vec, None)
        .expect("failed to create host function")
        .with_func::<(i32, i32, i32), i32, NeverType>("proc_string", proc_string, None)
        .expect("failed to create host function")
        .with_func::<(i32, i32, i32), i32, ShareFrames>(
            "load_video",
            load_video,
            Some(frames_arc.clone()),
        )
        .expect("failed to create host function")
        .with_func::<(i32, i32), i32, ShareFrames>("get_frame", get_frame, Some(frames_arc.clone()))
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
