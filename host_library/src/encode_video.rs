use std::{fs::File, io::prelude::*};

use ffmpeg::{
    codec,
    format::{self, Pixel},
    frame,
    software::scaling::{Context, Flags},
    util::frame::video::Video,
    Dictionary, Packet, Rational,
};

use crate::VideoInfo;

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
        let mut ost_time_bases = vec![Rational(0, 0); v_info.itcx_number_streams as _];

        let mut octx = format::output(&output_file).unwrap();

        let global_header = octx.format().flags().contains(format::Flags::GLOBAL_HEADER);
        let mut ost = octx.add_stream().unwrap();

        let codec = ffmpeg::encoder::find(codec::Id::H264).unwrap();

        println!("ENCODER CODEC : {:?}", codec.name());

        let mut encoder = ffmpeg::codec::Encoder::new(codec).unwrap().video().unwrap();

        println!("=======\nINIT encoder");
        println!("Settings {:#?}", v_info);

        encoder.set_height(v_info.height.0);
        encoder.set_width(v_info.width.0);
        encoder.set_aspect_ratio(v_info.aspect_ratio.0);
        encoder.set_format(v_info.format);
        encoder.set_frame_rate(v_info.frame_rate.0);
        encoder.set_time_base(Some(v_info.frame_rate.0.unwrap().invert()));
        println!("END INIT encoder\n=======");

        let mut dict = Dictionary::new();
        dict.set("preset", "medium");
        let ost_time_base = Rational(0, 0);

        println!("Start Open with encoder");
        let mut encoder: ffmpeg::encoder::Video = encoder
            .open_with(dict)
            .expect("error opening libx264 encoder with supplied settings");
        println!("END Open with encoder");
        println!("END Open with encoder");

        println!("==================");
        println!("Encoder Parameters");
        let enc_params = encoder.parameters();
        println!("ist_params {:?}", enc_params.id());
        println!("ist_params {:?}", enc_params.tag());
        println!("ist_params {:?}", enc_params.medium());
        println!("==================");

        let ost_time_base = Rational(0, 0);

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
        frames: &mut Vec<frame::Video>,
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

        for (idx, frame_rgb24) in frames.into_iter().enumerate() {
            frame_count += 1;
            let timestamp: Option<i64> = Some((idx * 1000) as i64);
            println!("Frame {idx} {:?}", timestamp);
            let mut frame_yuv420_p = Video::empty();

            scaler.run(&frame_rgb24, &mut frame_yuv420_p).unwrap();

            frame_yuv420_p.set_pts(timestamp);
            frame_yuv420_p.set_kind(ffmpeg::picture::Type::None);

            self.encoder.send_frame(&frame_yuv420_p).unwrap();
            // TODO SET PROPER STREAM INDEX
            // TODO Fix time scale
            // CHECK USING OST BASE TIME
            self.receive_and_process_encoded_packets(0);
        }

        // Send End of file information to Encoder and Output Context
        self.encoder.send_eof().unwrap();

        self.receive_and_process_encoded_packets(0);

        self.octx.write_trailer().unwrap();

        return Ok(());
    }

    fn receive_and_process_encoded_packets(&mut self, ost_index: usize) {
        let mut encoded = Packet::empty();
        while self.encoder.receive_packet(&mut encoded).is_ok() {
            encoded.set_stream(ost_index);

            println!(
                "rescale TS  SRC: {} - DST:{}",
                self.decoder_time_base, self.ost_time_bases[ost_index]
            );

            encoded.rescale_ts(self.decoder_time_base, self.ost_time_bases[ost_index]);

            let write_interleaved = encoded.write_interleaved(&mut self.octx);

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
