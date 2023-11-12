use std::{fs::File, io::prelude::*};

use ffmpeg::{
    codec,
    format::{self, Pixel},
    frame, option, picture,
    software::scaling::{Context, Flags},
    util::frame::video::Video,
    Dictionary, Packet, Rational,
};

use crate::VideoInfo;

enum EncoderError {
    FrameMapIsIncomplete(Vec<usize>), // TODO FILL THIS
}

pub(crate) struct VideoEncoder {
    // Encoder
    encoder: ffmpeg::encoder::Video,
    // Output Context
    octx: ffmpeg::format::context::output::Output,
    // Output Time Base
    ost_time_bases: Vec<Rational>,
    // Decoder Time Base
    decoder_time_base: Rational,
}

impl VideoEncoder {
    pub fn new(v_info: VideoInfo, output_file: &String) -> Self {
        let mut octx = format::output(&output_file).unwrap();

        let global_header = octx.format().flags().contains(format::Flags::GLOBAL_HEADER);
        let mut ost = octx.add_stream().unwrap();

        let codec = ffmpeg::encoder::find(codec::Id::H264).unwrap();
        // libx264rgb

        println!("ENCODER CODEC : {:?}", codec.name());

        let mut encoder = ffmpeg::codec::Encoder::new(codec).unwrap().video().unwrap();

        println!("=======\nINIT encoder");
        println!("Settings {:#?}", v_info);
        let frame_rate = v_info.frame_rate.0;

        // let mut ost_time_bases = vec![Rational(0, 0); v_info.itcx_number_streams as _];
        let mut ost_time_bases = Vec::new();
        ost_time_bases.push(frame_rate.map(|x| x.invert()).unwrap());

        encoder.set_height(v_info.height.0);
        encoder.set_width(v_info.width.0);
        encoder.set_aspect_ratio(v_info.aspect_ratio.0);
        encoder.set_format(v_info.format);
        encoder.set_frame_rate(frame_rate);
        encoder.set_time_base(frame_rate.map(|x| x.invert()));
        encoder.set_bit_rate(v_info.bitrate.0);
        encoder.set_max_bit_rate(v_info.max_bitrate.0);

        // encoder.set_max_b_frames(1);
        // Investigate more

        println!("END INIT encoder\n=======");

        let mut dict = Dictionary::new();
        dict.set("preset", "slow");
        // dict.set("preset", "medium");
        // dict.set("preset", "fast");

        // println!("Start Open with encoder");
        let mut encoder: ffmpeg::encoder::Video = encoder
            .open_with(dict)
            .expect("error opening libx264 encoder with supplied settings");
        // println!("END Open with encoder");

        println!("==================");
        println!("Encoder Parameters");
        let enc_params = encoder.parameters();
        println!("id : {:?}", enc_params.id());
        println!("tag: {:?}", enc_params.tag());
        println!("medium {:?}", enc_params.medium());
        println!("==================");

        ost.set_parameters(encoder.parameters());
        // let ost_time_base = ost_time_bases[ost_index as usize];

        if global_header {
            println!(
                "Setting Global header Flag : {:?}",
                codec::Flags::GLOBAL_HEADER
            );
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
        println!("Time Bases {:?}", ost_time_bases);

        VideoEncoder {
            encoder,
            octx,
            ost_time_bases,
            decoder_time_base: v_info.decoder_time_base,
        }
    }

    pub fn receive_and_process_decoded_frames(
        &mut self,
        frames: Vec<(&mut frame::Video, picture::Type, Option<i64>)>,
    ) -> Result<(), ()> {
        let mut frame_count = 0;

        // Write Every Frame out to encoder packet
        let mut scaler = Context::get(
            Pixel::RGB24,
            self.encoder.width(),
            self.encoder.height(),
            Pixel::YUV420P,
            self.encoder.width(),
            self.encoder.height(),
            Flags::BILINEAR,
        )
        .unwrap();

        // let mut last_frame_timestamp = option;
        for (idx, (out_frame, frame_type, frame_timestamp)) in frames.into_iter().enumerate() {
            frame_count += 1;
            // let timestamp: Option<i64> = Some((idx * 1000) as i64);
            println!(" {idx} {:?} , ", frame_timestamp);
            let mut frame_yuv420_p = Video::empty();
            scaler.run(&out_frame, &mut frame_yuv420_p).unwrap();

            // println!("SETTING PTS {:?}", timestamp);
            frame_yuv420_p.set_pts(frame_timestamp);
            frame_yuv420_p.set_kind(frame_type);
            // HARDCODED HARDCODED HARDCODED HARDCODED
            // HARDCODED HARDCODED HARDCODED HARDCODED
            // HARDCODED HARDCODED HARDCODED HARDCODED
            // HARDCODED HARDCODED HARDCODED HARDCODED
            // HARDCODED HARDCODED HARDCODED HARDCODED
            frame_yuv420_p.set_duration(Some(256));
            // HARDCODED HARDCODED HARDCODED HARDCODED
            // HARDCODED HARDCODED HARDCODED HARDCODED
            // HARDCODED HARDCODED HARDCODED HARDCODED
            // HARDCODED HARDCODED HARDCODED HARDCODED
            // HARDCODED HARDCODED HARDCODED HARDCODED

            self.encoder.send_frame(&frame_yuv420_p).unwrap();
            // TODO SET PROPER STREAM INDEX
            // TODO Fix time scale
            // CHECK USING OST BASE TIME
            self.receive_and_process_encoded_packets(0, frame_timestamp);
            // last_frame= ids;
        }

        // Send End of file information to Encoder and Output Context
        self.encoder.send_eof().unwrap();

        // self.receive_and_process_encoded_packets(0, Some((frame_count + 1) * 1000));
        self.receive_and_process_encoded_packets(0, None);

        if let Err(err) = self.octx.flush() {
            println!("Error: {}", err);
        };

        self.octx.write_trailer().unwrap();

        return Ok(());
    }

    fn receive_and_process_encoded_packets(&mut self, ost_index: usize, timestamp: Option<i64>) {
        let mut encoded_packet = Packet::empty();
        while self.encoder.receive_packet(&mut encoded_packet).is_ok() {
            encoded_packet.set_stream(ost_index);
            encoded_packet.set_time_base(Some(self.ost_time_bases[ost_index]));
            // encoded_packet.set_pts(timestamp);
            // encoded_packet.set_dts(timestamp);
            // encoded_packet.set_position(value);
            // println!(
            //     "rescale TS  SRC: {} - DST:{}",
            //     self.decoder_time_base, self.ost_time_bases[ost_index]
            // );
            encoded_packet.rescale_ts(self.decoder_time_base, self.ost_time_bases[ost_index]);
            // encoded_packet.set_dts(value);

            // let write_interleaved = encoded_packet.write(&mut self.octx);
            let write_interleaved = encoded_packet.write_interleaved(&mut self.octx);
            if let Err(err) = write_interleaved {
                println!("write_interleaved {:?}", err);
            };
        }
    }
}

// TODO: Remove debug function
pub fn _save_file(frame: &Video, index: usize) -> std::result::Result<(), std::io::Error> {
    let mut file = File::create(format!("frame{}.ppm", index))?;
    file.write_all(format!("P6\n{} {}\n255\n", frame.width(), frame.height()).as_bytes())?;
    file.write_all(frame.data(0))?;
    Ok(())
}
