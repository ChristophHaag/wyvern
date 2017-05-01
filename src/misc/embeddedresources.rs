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

use std::collections::HashMap;

pub trait EmbeddedResources {
    /// Indicates whether there are embedded resources or not
    fn use_me(&self) -> bool;

    /// Accessor for the embedded resources
    fn resources(&self) -> &HashMap<&'static str, &'static str>;
}

// The following is an "empty" implementation of the trait for applications
// to use if they don't want to use the resource-embedding feature.
//
pub struct NoEmbedded {
    pub use_me: bool,
    pub resources: HashMap<&'static str, &'static str>,
}

impl NoEmbedded {
    pub fn new() -> NoEmbedded {
        let res = NoEmbedded {
            use_me: false,
            resources: HashMap::new(),
        };

        res
    }
}

impl EmbeddedResources for NoEmbedded {
    fn use_me(&self) -> bool {
        self.use_me
    }

    fn resources(&self) -> &HashMap<&'static str, &'static str> {
        &self.resources
    }
}
