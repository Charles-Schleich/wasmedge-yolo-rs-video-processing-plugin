mod plugin {
    // use crate::MyStr;

    // #[link(wasm_import_module = "yolo-video-proc")]
    // extern "C" {
    //     pub fn hello(a: String,x: String, y: MyStr) -> i32;
    // }

    #[link(wasm_import_module = "yolo-video-proc")]
    extern "C" {
        pub fn proc_vec(ext_ptr: i32, buf_len: i32, capacity: i32) -> i32;
        pub fn proc_string(ext_ptr: i32, buf_len: i32, capacity: i32) -> i32;
        // pub fn hello(a: i32,x: i32) -> i32;
        // pub fn imencode(ext_ptr: *const u8, ext_len: usize, m: mat_key, buf_ptr: *const u8, buf_len: usize) -> ();
    }
}

// MyString is similar to Rust built-in String
#[derive(Debug)]
pub struct MyString {
    pub s: String,
}

// MyStr is similar to Rust built-in string slice, namely str
#[derive(Debug)]
pub struct MyStr<'a> {
    pub s: &'a str,
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
    // println!("Call Proc String");
    // call_proc_string();

    println!("Call Proc Vec");
    call_proc_vec();

    Ok(())
}
