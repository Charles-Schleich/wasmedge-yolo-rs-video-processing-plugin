mod plugin {
    #[link(wasm_import_module = "yolo-video-proc")]
    extern "C" {
        pub fn proc_vec(ext_ptr: i32, buf_len: i32, capacity: i32) -> i32;
        pub fn proc_string(ext_ptr: i32, buf_len: i32, capacity: i32) -> i32;
        pub fn load_video(str_ptr: i32, str_len: i32, str_capacity: i32) -> i32;
        pub fn proc_video(buf_ptr: i32, buf_len: i32, buf_capacity: i32) -> i32;
        pub fn get_frame(frame_index: i32, frame_ptr: i32) -> i32;
    }
}

fn process_video(mut filename: String) -> Vec<image::DynamicImage> {
    let num_frames = unsafe {
        plugin::load_video(
            filename.as_mut_ptr() as usize as i32,
            filename.len() as i32,
            filename.capacity() as i32,
        )
    };

    // let num_frames = unsafe {
    //     plugin::load_video(
    //         filename.as_mut_ptr() as usize as i32,
    //         filename.len() as i32,
    //         filename.capacity() as i32,
    //     )
    // };

    println!("Woop woop {}", num_frames);
    let mut frame = "10".to_string();

    for idx in 0..num_frames {
        println!("------ Run for frame {}", idx);

        unsafe {
            plugin::get_frame(
                idx,
                frame.as_mut_ptr() as usize as i32,
            )
        };
        println!("------ Run for frame {}", idx);
    }

    // let mut images = Vec::<image::DynamicImage>::with_capacity(num_frames as usize);
    // let buf_len = images.len() as i32;
    // let buf_capacity = images.capacity() as i32;
    // let buf_ptr_raw = images.as_mut_ptr() as usize as i32;

    // let x = unsafe { plugin::proc_video(buf_ptr_raw, buf_len, buf_capacity) };
    // images should contain the images here.

    todo!();
}

fn call_proc_vec() {
    let mut buf: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let buf_len = buf.len() as i32;
    let buf_capacity = buf.capacity() as i32;
    let buf_ptr_raw = buf.as_mut_ptr() as usize as i32;
    println!("Before Function Call '{:?}'", buf);
    let y = unsafe { plugin::proc_vec(buf_ptr_raw, buf_len, buf_capacity) };
    println!("After Function Call '{:?}'", buf);
}

fn call_proc_string() {
    let mut s = "hello plugin".to_string();
    println!("Before Function Call '{}'", s);
    let y = unsafe {
        plugin::proc_string(
            s.as_mut_ptr() as usize as i32,
            s.len() as i32,
            s.capacity() as i32,
        )
    };
    println!("After Function Call {}", s);
    println!("Function Output {}", y);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Call Proc Vec");
    // call_proc_vec();
    process_video("./times_square.mp4".to_string());
    // process_video("./demo_video/out.mp4".to_string());
    // process_video("./demo_video/out_YUV.mp4".to_string());

    Ok(())
}
