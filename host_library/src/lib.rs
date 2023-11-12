use std::{
    io::Write,
    sync::{Arc, Mutex},
};

mod decode_video;
mod encode_video;

use ffmpeg::{dictionary, format::Pixel, frame, picture, Codec, Rational};

use wasmedge_sdk::{
    error::HostFuncError,
    host_function,
    plugin::{ffi, PluginDescriptor, PluginModuleBuilder, PluginVersion},
    Caller, NeverType, WasmValue,
};

use std::fmt::Debug;

use log::debug;

#[derive(Debug, Copy, Clone)]
pub struct Width(pub u32);
#[derive(Debug, Copy, Clone)]
pub struct Height(pub u32);
#[derive(Debug, Copy, Clone)]
pub struct AspectRatio(pub Rational);
#[derive(Debug, Copy, Clone)]
pub struct FrameRate(pub Option<Rational>);

#[derive(Debug, Copy, Clone)]
pub struct BitRate(pub usize);
#[derive(Debug, Copy, Clone)]
pub struct MaxBitRate(pub usize);

#[derive(Clone)]
pub struct VideoInfo {
    pub codec: Codec,
    pub format: Pixel,
    pub width: Width,
    pub height: Height,
    pub aspect_ratio: AspectRatio,
    pub frame_rate: FrameRate,
    pub input_stream_meta_data: dictionary::Owned,
    pub itcx_number_streams: u32,
    pub decoder_time_base: Rational,
    pub bitrate: BitRate,
    pub max_bitrate: MaxBitRate,
}

impl Debug for VideoInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VideoInfo")
            .field("codec", &self.codec.name())
            .field("format", &self.format)
            .field("width", &self.width.0)
            .field("height", &self.height.0)
            .field("aspect_ratio", &self.aspect_ratio.0)
            .field("frame_rate", &self.frame_rate.0)
            .field("input_stream_meta_data", &self.input_stream_meta_data)
            .field("itcx_number_streams", &self.itcx_number_streams)
            .finish()
    }
}

impl VideoInfo {
    pub fn new(
        codec: Codec,
        format: Pixel,
        width: Width,
        height: Height,
        aspect_ratio: AspectRatio,
        frame_rate: FrameRate,
        input_stream_meta_data: dictionary::Owned,
        itcx_number_streams: u32,
        decoder_time_base: Rational,
        bitrate: BitRate,
        max_bitrate: MaxBitRate,
    ) -> Self {
        VideoInfo {
            codec,
            format,
            width,
            height,
            aspect_ratio,
            frame_rate,
            input_stream_meta_data,
            itcx_number_streams,
            decoder_time_base,
            bitrate,
            max_bitrate,
        }
    }

    pub fn width(&self) -> u32 {
        self.width.0
    }

    pub fn height(&self) -> u32 {
        self.height.0
    }
}

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
fn load_video_to_host_memory(
    caller: Caller,
    args: Vec<WasmValue>,
    data: &mut Arc<Mutex<FramesMap>>, // data: &mut Frames,
) -> Result<Vec<WasmValue>, HostFuncError> {
    debug!("Load_video");
    let data_guard = data.lock().unwrap();
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

    let res = match decode_video::dump_frames(&filename) {
        Ok((frames, video_info)) => {
            debug!("Input Frame Count {}", frames.len());
            if frames.len() > 0 {
                unsafe {
                    *width_ptr_main_memory = frames[0].input_frame.width();
                    *height_ptr_main_memory = frames[0].input_frame.height();
                }
            }

            // *(data_guard) = frames;
            let mut vid_gaurd = data_guard;
            vid_gaurd.video_info = Some(video_info);
            vid_gaurd.frames = frames;

            println!("Data Len {:?}", vid_gaurd.frames.len());

            Ok(vec![WasmValue::from_i32(vid_gaurd.frames.len() as i32)])
        }
        // TODO: Make Error more clear
        Err(err) => Err(HostFuncError::User(1)),
    };

    std::mem::forget(filename); // Need to forget x otherwise we get a double free
    res
}

#[host_function]
fn get_frame(
    caller: Caller,
    args: Vec<WasmValue>,
    data: &mut Arc<Mutex<FramesMap>>,
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
    // TODO proper Handling of errors
    let image_ptr_wasm_memory = main_memory
        .data_pointer_mut(image_buf_ptr as u32, image_buf_len as u32)
        .expect("Could not get Data pointer");

    let mut vec =
        unsafe { Vec::from_raw_parts(image_ptr_wasm_memory, image_buf_len, image_buf_capacity) };

    if let Some(frame) = data_guard.frames.get(idx as usize) {
        debug!("LIB data {:?}", frame.input_frame.data(0).len());
        vec.copy_from_slice(frame.input_frame.data(0));
    } else {
        // TODO return Error
        todo!("Return error if frame does not exist");
    };

    std::mem::forget(vec); // Need to forget x otherwise we get a double free
    Ok(vec![WasmValue::from_i32(1)])
}

#[host_function]
fn write_frame(
    caller: Caller,
    args: Vec<WasmValue>,
    data: &mut Arc<Mutex<FramesMap>>,
) -> Result<Vec<WasmValue>, HostFuncError> {
    debug!("write_frame");

    let mut main_memory = caller.memory(0).unwrap();
    let idx = args[0].to_i32() as usize;
    let image_buf_ptr = args[1].to_i32();
    let image_buf_len = args[2].to_i32() as usize;

    let mut data_mtx = data.lock().unwrap();
    // TODO: Proper Error Handling Video info
    let video_info = data_mtx.video_info.clone().unwrap();

    // TODO proper Handling of errors
    let image_ptr_wasm_memory = main_memory
        .data_pointer_mut(image_buf_ptr as u32, image_buf_len as u32)
        .expect("Could not get Data pointer");

    let vec = unsafe {
        Vec::from_raw_parts(
            image_ptr_wasm_memory,
            image_buf_len,
            (video_info.width() * video_info.height() * 3) as usize,
        )
    };

    debug!(
        "BUFFER SIZE {}",
        video_info.width() * video_info.height() * 3
    );

    let mut video_frame = frame::Video::new(
        ffmpeg::format::Pixel::RGB24,
        video_info.width.0,
        video_info.height.0,
    );

    {
        let data = video_frame.data_mut(0);
        data.copy_from_slice(&vec);
    }

    println!("Writing Frame {idx}");
    if let Some(frame_map) = data_mtx.frames.get_mut(idx) {
        frame_map.output_frame = Some(video_frame);
    } else {
        return Ok(vec![WasmValue::from_i32(1)]);
    };

    std::mem::forget(vec); // Need to forget x otherwise we get a double free
    Ok(vec![WasmValue::from_i32(0)])
}

#[host_function]
fn assemble_output_frames_to_video(
    caller: Caller,
    args: Vec<WasmValue>,
    data: &mut Arc<Mutex<FramesMap>>,
) -> Result<Vec<WasmValue>, HostFuncError> {
    debug!("assemble_video");
    let mut data_mg = data.lock().unwrap();
    let mut main_memory = caller.memory(0).unwrap();

    let filename_ptr = args[0].to_i32();
    let filename_len = args[1].to_i32();
    let filaname_capacity = args[2].to_i32();

    // TODO Proper Error Handling
    // TODO remove clone
    let video_info = data_mg.video_info.clone().unwrap();

    // TODO proper Handling of errors
    let filename_ptr_main_memory = main_memory
        .data_pointer_mut(filename_ptr as u32, filename_len as u32)
        .expect("Could not get Data pointer");

    let output_file: String = unsafe {
        String::from_raw_parts(
            filename_ptr_main_memory,
            filename_len as usize,
            filaname_capacity as usize,
        )
    };

    let video_struct = &mut (*data_mg);
    let frames = &mut video_struct.frames;
    let video_info = video_struct.video_info.clone().unwrap();

    // Check Frames have all been Written
    // Save Indexes of frames that have not been written
    let (frames, missing_frames) = frames.into_iter().enumerate().fold(
        (Vec::new(), Vec::new()),
        |(mut iter_frames, mut iter_missing), (idx, frame_map)| {
            match frame_map.output_frame.as_mut() {
                Some(fr) => iter_frames.push((fr, frame_map.frame_type, frame_map.timestamp)),
                None => iter_missing.push(idx),
            };
            (iter_frames, iter_missing)
        },
    );

    if missing_frames.len() > 0 {
        // TODO: Return correct Error stating missing frames based on Error enum
        println!("ERROR MISSING FRAMES {:?} ", missing_frames);
        return Ok(vec![WasmValue::from_i32(1)]);
    }

    let mut video_encoder = encode_video::VideoEncoder::new(video_info, &output_file);

    if let Err(err) = video_encoder.receive_and_process_decoded_frames(frames) {
        println!("Encode stream ERROR {:?}", err);
    };

    std::mem::forget(output_file); // Need to forget x otherwise we get a double free

    Ok(vec![WasmValue::from_i32(1)])
}

#[derive(Clone)]
struct FramesMap {
    frames: Frames,
    video_info: Option<VideoInfo>,
}

// How to do this ?
// Vec <FrameMap?>
// HashMap ?
// Use Timestamp as a Key ?
// Check All frames are written ?

#[derive(Clone)]
pub struct FrameMap {
    input_frame: frame::Video,
    // Input Frame Type
    frame_type: picture::Type,
    // Input Frame Timestamp
    timestamp: Option<i64>,

    // Option as we are not sure if it has been processed yet or not
    output_frame: Option<frame::Video>,
}

type Frames = Vec<FrameMap>;
type ShareFrames = Arc<Mutex<FramesMap>>;

/// Defines Plugin module instance
unsafe extern "C" fn create_test_module(
    _arg1: *const ffi::WasmEdge_ModuleDescriptor,
) -> *mut ffi::WasmEdge_ModuleInstanceContext {
    let module_name = "yolo-video-proc";

    let video_frames = FramesMap {
        frames: Vec::new(),
        video_info: None,
    };

    let video_frames_arc = Box::new(Arc::new(Mutex::new(video_frames)));

    // TODO Wrap i32's in Struct to avoid misuse / mixups
    type Width = i32;
    type Height = i32;

    let plugin_module = PluginModuleBuilder::<NeverType>::new()
        // .with_func::<(ExternRef, ExternRef), i32, NeverType>("hello", hello, None)
        .with_func::<(i32, i32, i32), i32, NeverType>("proc_vec", proc_vec, None)
        .expect("failed to create host function")
        .with_func::<(i32, i32, i32), i32, NeverType>("proc_string", proc_string, None)
        .expect("failed to create host function")
        .with_func::<(i32, i32, i32, Width, Height), i32, ShareFrames>(
            "load_video_to_host_memory",
            load_video_to_host_memory,
            Some(video_frames_arc.clone()),
        )
        .expect("failed to create load_video_to_host_memory host function")
        .with_func::<(i32, i32, i32, i32), i32, ShareFrames>(
            "get_frame",
            get_frame,
            Some(video_frames_arc.clone()),
        )
        .expect("failed to create get_frame host function")
        .with_func::<(i32, i32, i32), i32, ShareFrames>(
            "write_frame",
            write_frame,
            Some(video_frames_arc.clone()),
        )
        .expect("failed to create write_frame host function")
        .with_func::<(i32, i32, i32), i32, ShareFrames>(
            "assemble_output_frames_to_video",
            assemble_output_frames_to_video,
            Some(video_frames_arc.clone()),
        )
        .expect("failed to create assemble_output_frames_to_video host function")
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
