use std::{fs::File, io::prelude::*};

use ffmpeg::{
    codec,
    format::{self},
    frame,
    util::frame::video::Video,
    Dictionary, Packet, Rational,
};

use crate::{Frames, VideoInfo};

pub fn encode_frames(
    output_file: &String,
    frames: &mut Vec<frame::Video>,
    v_info: VideoInfo,
) -> Result<Frames, ffmpeg::Error> {
    // v_info.itcx_number_streams
    let mut ost_time_bases = vec![Rational(0, 0); v_info.itcx_number_streams as _];

    let mut octx = format::output(&output_file).unwrap();

    let global_header = octx.format().flags().contains(format::Flags::GLOBAL_HEADER);
    let mut ost = octx.add_stream()?;

    let codec = ffmpeg::encoder::find(codec::Id::H264).unwrap();

    println!("ENCODER CODEC : {:?}", codec.name());

    let mut encoder = ffmpeg::codec::Encoder::new(codec)?.video()?;
    println!("INIT encoder");

    encoder.set_height(v_info.height.0);
    encoder.set_width(v_info.width.0);
    encoder.set_aspect_ratio(v_info.aspect_ratio.0);
    encoder.set_format(v_info.format);
    encoder.set_frame_rate(v_info.frame_rate.0);
    encoder.set_time_base(Some(v_info.frame_rate.0.unwrap().invert()));
    println!("END INIT encoder");

    let mut dict = Dictionary::new();
    dict.set("preset", "medium");
    let ost_time_base = Rational(0, 0);

    println!("Start Open with encoder");
    let mut encoder: ffmpeg::encoder::Video = encoder
        .open_with(dict)
        .expect("error opening libx264 encoder with supplied settings");
    println!("END Open with encoder");

    ost.set_parameters(encoder.parameters());
    // let ost_time_base = ost_time_bases[ost_index as usize];

    if global_header {
        encoder.set_flags(codec::Flags::GLOBAL_HEADER);
    }

    println!("==========================");
    println!("Write output context");
    octx.set_metadata(v_info.input_stream_meta_data);
    format::context::output::dump(&octx, 0, Some(&output_file));
    octx.write_header().unwrap();
    println!("==========================");

    for (ost_index, _) in octx.streams().enumerate() {
        println!(
            "OST_TB: {ost_index} {}",
            octx.stream(ost_index as _).unwrap().time_base().unwrap()
        );
        ost_time_bases[ost_index] = octx.stream(ost_index as _).unwrap().time_base().unwrap();
    }
    println!("OST_TB ALL: {:?}", ost_time_bases);
    println!("=========");

    println!("Encoder : {:?}", v_info.codec.name());

    receive_and_process_decoded_frames(frames, &mut octx, &mut encoder, ost_time_base);

    // encoder.
    // TODO:Continue from here
    // ffmpeg::encoder::Video::

    todo!();
}

fn receive_and_process_decoded_frames(
    frames: &mut Vec<frame::Video>,
    octx: &mut format::context::Output,
    encoder: &mut ffmpeg::encoder::Video,
    ost_time_base: Rational,
) {
    let mut frame_count = 0;
    let decoder_time_base = ost_time_base;
    // Write Every Frame out to encoder packet
    for (idx, mut frame) in frames.into_iter().enumerate() {
        frame_count += 1;
        // let timestamp = frame.timestamp();
        let timestamp: Option<i64> = Some((idx * 1000) as i64);

        println!("Frame {idx} {:?}", timestamp);
        println!("TS    {:?}", timestamp);
        println!("OST T Base: {:?}", ost_time_base);
        println!("Decoder T B: {:?}", decoder_time_base);

        frame.set_pts(timestamp);
        frame.set_kind(ffmpeg::picture::Type::None);

        encoder.send_frame(frame).unwrap();

        // TODO SET PROPER STREAM INDEX
        // CHECK USING OST BASE TIME
        receive_and_process_encoded_packets(encoder, 0, octx, ost_time_base, ost_time_base);
    }
}

fn receive_and_process_encoded_packets(
    encoder: &mut ffmpeg::encoder::Video,
    ost_index: usize,
    octx: &mut format::context::Output,
    ost_time_base: Rational,
    decoder_time_base: Rational,
) {
    let mut encoded = Packet::empty();
    while encoder.receive_packet(&mut encoded).is_ok() {
        encoded.set_stream(ost_index);
        encoded.rescale_ts(decoder_time_base, ost_time_base);
        let write_interleaved = encoded.write_interleaved(octx);
        if let Err(err) = write_interleaved {
            println!("write_interleaved {:?}", err);
        };
    }
}

pub fn save_file(frame: &Video, index: usize) -> std::result::Result<(), std::io::Error> {
    let mut file = File::create(format!("frame{}.ppm", index))?;
    file.write_all(format!("P6\n{} {}\n255\n", frame.width(), frame.height()).as_bytes())?;
    file.write_all(frame.data(0))?;
    Ok(())
}
