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
use std::fs;
use std::fs::*;
use std::io;
use std::io::prelude::*;
use std::error::Error;
use std::time::SystemTime;

use misc::embeddedresources::*;

/// Read the entire contents of a file into a string
///
/// Based on the code for opening and reading from files on Rust By Example
///
/// embedded: The embedded resources object
/// filename: The name of the file to read in
pub fn read_text_file(embedded: Option<&EmbeddedResources>, filename: &str) -> String {
    let mut contents = String::new();
    let mut use_embedded = false;

    match embedded {
        Some(ref embedded) => use_embedded = embedded.use_me(),
        None => (),
    };

    if use_embedded {
        // Use embedded resources
        match embedded {
            Some(ref embedded) => contents = embedded.resources()[filename].to_string(),
            None => (),
        };
    } else {
        // Use the resource files from the source tree

        // Create a path to the desired file
        let path = Path::new(&filename);
        let display = path.display();

        // Open the path in read-only mode
        let mut file = match File::open(&path) {
            Err(why) => panic!("Failed to open file {}: {}", display, why.description()),
            Ok(file) => file,
        };

        // Read the file contents into a string
        match file.read_to_string(&mut contents) {
            Err(why) => panic!("Failed to read file {}: {}", display, why.description()),
            Ok(_) => (),
        }
    }

    contents
}

/// Get the last modification timestamp for a file
///
/// filename: The filename to get the last modified time from
///
/// Returns the system time of the last modification
pub fn get_last_modification_timestamp(filename: &str) -> Result<SystemTime, io::Error> {
    let path = Path::new(filename);
    let data = fs::metadata(&path)?;

    data.modified()
}

/// Convert a value representing an 8-bit char to a printable pair
///
/// c: Input character
///
/// Returns a character safe to print
fn to_hex(c: u8) -> char {
    if c <= 9 {
        return (c + '0' as u8) as char;
    }
    if c <= 15 {
        return ((c - 10) + 'a' as u8) as char;
    }

    debug_assert!(false);
    '?'
}


/// Convert a value representing an 8-bit char to a printable pair
///
/// c: Input character
///
/// Returns a character safe to print
fn print_hex(c: u32) -> (char, char) {
    if c == 0xffff {
        (' ', ' ')
    } else {
        (to_hex(((c >> 4) & 0xf) as u8), to_hex(((c & 0xf) as u8)))
    }
}

/// Convert a byte char to a safe-to-print character
///
/// c: Input character
///
/// Returns a character safe to print
fn safe_char(c: u32) -> char {
    if c >= 32 && c <= 126 {
        return c as u8 as char;
    }

    if c == 0xffff {
        return ' ';
    }

    '.'
}

/// A pretty-printer for a vector of bytes
///
/// bytes: The bytes to dump
pub fn dump_byte_vector(bytes: &Vec<u8>) {
    let mut addr = 0;
    let mut i = 0;
    let mut buf: Vec<u32> = vec![];
    buf.reserve(4);

    for byte in bytes.iter() {
        buf.push(*byte as u32);
        i += 1;
        if i == 16 {
            print!("{:08x}: ", addr);
            for j in 0..4 {
                print!("{:02x} {:02x} {:02x} {:02x}  ",
                       buf[j * 4],
                       buf[j * 4 + 1],
                       buf[j * 4 + 2],
                       buf[j * 4 + 3]);
            }
            for j in 0..4 {
                print!("{}{}{}{} ",
                       safe_char(buf[j * 4]),
                       safe_char(buf[j * 4 + 1]),
                       safe_char(buf[j * 4 + 2]),
                       safe_char(buf[j * 4 + 3]));
            }
            println!("");
            buf.clear();
            i = 0;
            addr += 16;
        }
    }
    if i != 0 {
        for _ in 0..(16 - i) {
            buf.push(0xffff);
        }
        print!("{:08x}: ", addr);
        for j in 0..4 {
            let (c1, c2) = print_hex(buf[j * 4]);
            let (c3, c4) = print_hex(buf[j * 4 + 1]);
            let (c5, c6) = print_hex(buf[j * 4 + 2]);
            let (c7, c8) = print_hex(buf[j * 4 + 3]);
            print!("{}{} {}{} {}{} {}{}  ", c1, c2, c3, c4, c5, c6, c7, c8);
        }
        for j in 0..4 {
            print!("{}{}{}{} ",
                   safe_char(buf[j * 4]),
                   safe_char(buf[j * 4 + 1]),
                   safe_char(buf[j * 4 + 2]),
                   safe_char(buf[j * 4 + 3]));
        }
        println!("");
    }
}

/// A pretty-printer for the first elements of a vector of floats
///
/// values: The values to dump
/// number: The number of elements to dump
/// columns: The number of columns to display the data in
pub fn dump_float_vector(values: &Vec<f32>, number: usize, columns: usize) {
    let mut i = 0;
    for j in 0..number {
        print!("{:>14.*}", 4, values[j]);
        i += 1;
        if i == columns {
            println!("");
            i = 0;
        } else {
            print!(" ");
        }
    }
}

/// Read the entire contents of a file into a byte vector
///
/// filename: The name of the file to read in
/// dump: true to print the bytecode to the console, false otherwise
pub fn read_binary_file(filename: &str, dump: bool) -> Result<Vec<u8>, io::Error> {
    let path = Path::new(filename);
    let mut file = File::open(&path)?;
    let mut bytecode: Vec<u8> = vec![];
    file.read_to_end(&mut bytecode)?;

    if dump {
        dump_byte_vector(&bytecode);
    }

    Ok(bytecode)
}

/// Write the specified contents to a new file
///
/// contents: What to write
/// filename: Where
pub fn write_entire_file(contents: &str, filename: &str) -> Result<(), io::Error> {
    let mut output_file = File::create(filename)?;
    output_file.write_all(contents.as_bytes())?;

    Ok(())
}
