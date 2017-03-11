# wyvern
Experimental OpenGL/Vulkan application framework and utilities library written in Rust

# Dependencies

Binaries of the dependencies are distributed with this project, including
those for OpenHMD and HIDAPI.  The intent is that the executable is statically
linked against all non-system dependencies, but this is not currently achieved.

Windows has a couple of dependencies.  For building and running on Linux the
appropriate HIDAPI, X11, and OpenGL packages need to be installed.

## Windows

Install the following and ensure that your PATH is set up appropriately:

* Microsoft linker and runtimes
* CMake for building various Rust crates

## Fedora

Please install the following packages on Fedora to build and run:

* cmake
* libX11-devel
* libXcursor-devel
* libXi-devel
* libXinerama-devel
* libXmu-devel
* libXrandr-devel

For running with the Vulkan renderer, please also install the following
package and its prerequisites to get Vulkan configured for using the
validation layers:

* vulkan-devel

It ought to be possible to install from the official Vulkan SDK, but
the install script appears to for Debian-based distros only, and (as
of 1.0.30) also appears to be missing any of the installation packages
referred to.

# Conventions

For consistency, the intention is to note and/or follow these points and
conventions.  It is entirely possible that there are still violations of
these within the application as it is very easy to mistakenly counter one
violation by introducing another!

* Rust arrays, as for those in C, are row-major.
* OpenGL matrices, as in Fortran, are column-major.
* This discrepancy is dealt with by producing row-major matrices in Rust
  and pre-multiplying to perform a transformation (v' = Mv).  Other
  applications choose to produce column-major matrices when using
  OpenGL, and then post-multiply (v' = vM).
* The cross-product should be defined by any application or library such
  that the handedness of the coordinate system in use is preserved.
* The OpenGL fixed function pipeline uses right-handed coordinates
  everywhere except for clip space.
  * The frustum projection matrix is constructed to introduce the flip.
  * I believe the reasons for this to be historical.
  * And it certainly introduces a great deal of confusion!

# Acknowledgements

I would like to extend my thanks explicitly to the following projects:

* GLFW [link] (http://www.glfw.org/)
* glfw-rs [link] (https://github.com/PistonDevelopers/glfw-rs)
* VulkanTutorial by Alexander Overvoorde [link] (https://vulkan-tutorial.com/)
* All of the developers of the multitude of Rust crates available on crates.io

And, of course, many thanks to the Rust and OpenGL/GLSL/Vulkan/SPIR-V/Khronos
engineers.

For license texts, see the other-licenses directory.
