// Copyright (c) 2016-2017 Bruce Stenning. All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
//
// 1. Redistributions of source code must retain the above copyright
//    notice, this list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright
//    notice, this list of conditions and the following disclaimer in the
//    documentation and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its
//    contributors may be used to endorse or promote products derived
//    from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
// FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
// COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
// INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
// BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS
// OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED
// AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF
// THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH
// DAMAGE.

use std::path::Path;
use image;
use image::*;

pub struct Image {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl Image {
    /// Return the width of the image
    pub fn get_width(&self) -> u32 {
        self.width
    }

    /// Return the height of the image
    pub fn get_height(&self) -> u32 {
        self.height
    }

    /// Return a clone of the pixel data of the image
    ///
    /// TODO: Consider using an Rc or an Arc to avoid copying the data
    pub fn get_data(&self) -> Vec<u8> {
        self.data.clone()
    }

    /// Create an Image object from the PNG file specified
    ///
    /// filename: The name of the PNG file to load into the image
    pub fn load_from_png(filename: &str) -> Image {
        let img = image::open(&Path::new(filename)).unwrap();
        let (width, height) = img.dimensions();

        // Assume the input does not have an alpha channel and add one
        //
        let mut reformatted: Vec<u8> = Vec::with_capacity(img.raw_pixels().len() * 4 / 3);
        let mut i = 0;
        for byte in img.raw_pixels() {
            reformatted.push(byte);
            i += 1;
            if i == 3 {
                reformatted.push(0);
                i = 0;
            }
        }

        Image {
            width: width,
            height: height,
            data: reformatted,
        }
    }

    /// Create a new Image from raw RGB data
    ///
    /// width: The image width
    /// height: The image height
    /// data: The raw RGB data
    pub fn create_from_raw_data(width: u32, height: u32, data: &Vec<u8>) -> Image {
        let mut flipped: Vec<u8> = vec![];

        for y in 0..height {
            for x in 0..(width * 3) {
                flipped.push(data[((height - 1 - y) * width * 3 + x) as usize]);
            }
        }

        Image {
            width: width,
            height: height,
            data: flipped,
        }
    }

    /// Write the image to a file
    ///
    /// filename: The filename to use for the image on disk
    pub fn save_to(&self, filename: &str) {
        image::save_buffer(&Path::new(filename),
                           self.data.as_slice(),
                           self.width,
                           self.height,
                           image::RGB(8))
            .unwrap();
    }
}
