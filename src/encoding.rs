use crate::{error_or, ffi, utilities::rcp_safe, Result};
use std::mem;

/// Encodes index data into an array of bytes that is generally much smaller (<1.5 bytes/triangle)
/// and compresses better (<1 bytes/triangle) compared to original.
///
/// For maximum efficiency the index buffer being encoded has to be optimized for vertex cache and
/// vertex fetch first.
pub fn encode_index_buffer(indices: &[u32], vertex_count: usize) -> Result<Vec<u8>> {
    let bounds = unsafe { ffi::meshopt_encodeIndexBufferBound(indices.len(), vertex_count) };
    let mut result: Vec<u8> = vec![0; bounds];
    let size = unsafe {
        ffi::meshopt_encodeIndexBuffer(
            result.as_mut_ptr() as *mut ::std::os::raw::c_uchar,
            result.len(),
            indices.as_ptr() as *const ::std::os::raw::c_uint,
            indices.len(),
        )
    };
    result.resize(size, 0u8);
    Ok(result)
}

/// Decodes index data from an array of bytes generated by `encode_index_buffer`.
/// The decoder is safe to use for untrusted input, but it may produce garbage
/// data (e.g. out of range indices).
pub fn decode_index_buffer<T: Clone + Default + Sized>(
    encoded: &[u8],
    index_count: usize,
) -> Result<Vec<T>> {
    const fn assert_valid_size<T: Sized>() {
        assert!(
            mem::size_of::<T>() == 2 || mem::size_of::<T>() == 4,
            "size of result type must be 2 or 4 bytes wide"
        );
    }

    assert_valid_size::<T>();

    let mut result: Vec<T> = vec![Default::default(); index_count];
    let result_code = unsafe {
        ffi::meshopt_decodeIndexBuffer(
            result.as_mut_ptr().cast(),
            index_count,
            mem::size_of::<T>(),
            encoded.as_ptr(),
            encoded.len(),
        )
    };

    error_or(result_code, result)
}

/// Encodes vertex data into an array of bytes that is generally smaller and compresses better
/// compared to original.
///
/// This function works for a single vertex stream; for multiple vertex streams,
/// call `encode_vertex_buffer` for each stream.
pub fn encode_vertex_buffer<T>(vertices: &[T]) -> Result<Vec<u8>> {
    let bounds =
        unsafe { ffi::meshopt_encodeVertexBufferBound(vertices.len(), mem::size_of::<T>()) };
    let mut result: Vec<u8> = vec![0; bounds];
    let size = unsafe {
        ffi::meshopt_encodeVertexBuffer(
            result.as_mut_ptr() as *mut ::std::os::raw::c_uchar,
            result.len(),
            vertices.as_ptr() as *const ::std::os::raw::c_void,
            vertices.len(),
            mem::size_of::<T>(),
        )
    };
    result.resize(size, 0u8);
    Ok(result)
}

/// Decodes vertex data from an array of bytes generated by `encode_vertex_buffer`.
/// The decoder is safe to use for untrusted input, but it may produce garbage data.
pub fn decode_vertex_buffer<T: Clone + Default>(
    encoded: &[u8],
    vertex_count: usize,
) -> Result<Vec<T>> {
    let mut result: Vec<T> = vec![Default::default(); vertex_count];
    let result_code = unsafe {
        ffi::meshopt_decodeVertexBuffer(
            result.as_mut_ptr() as *mut ::std::os::raw::c_void,
            vertex_count,
            mem::size_of::<T>(),
            encoded.as_ptr() as *const ::std::os::raw::c_uchar,
            encoded.len(),
        )
    };

    error_or(result_code, result)
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct EncodeHeader {
    pub magic: [u8; 4], // OPTM

    pub group_count: u32,
    pub vertex_count: u32,
    pub index_count: u32,
    pub vertex_data_size: u32,
    pub index_data_size: u32,

    pub pos_offset: [f32; 3],
    pub pos_scale: f32,
    pub uv_offset: [f32; 2],
    pub uv_scale: [f32; 2],

    pub reserved: [u32; 2],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct EncodeObject {
    pub index_offset: u32,
    pub index_count: u32,
    pub material_length: u32,
    pub reserved: u32,
}

pub fn calc_pos_offset_and_scale(positions: &[f32]) -> ([f32; 3], f32) {
    use std::f32::MAX;

    let pos_offset = positions
        .chunks(3)
        .fold([MAX, MAX, MAX], |result, position| {
            [
                result[0].min(position[0]),
                result[1].min(position[1]),
                result[2].min(position[2]),
            ]
        });

    let pos_scale = positions.chunks(3).fold(0f32, |result, position| {
        result
            .max(position[0] - pos_offset[0])
            .max(position[1] - pos_offset[1])
            .max(position[2] - pos_offset[2])
    });

    (pos_offset, pos_scale)
}

pub fn calc_pos_offset_and_scale_inverse(positions: &[f32]) -> ([f32; 3], f32) {
    let (pos_offset, pos_scale) = calc_pos_offset_and_scale(positions);
    let pos_scale_inverse = rcp_safe(pos_scale);
    (pos_offset, pos_scale_inverse)
}

pub fn calc_uv_offset_and_scale(coords: &[f32]) -> ([f32; 2], [f32; 2]) {
    use std::f32::MAX;

    let uv_offset = coords.chunks(2).fold([MAX, MAX], |result, coord| {
        [result[0].min(coord[0]), result[1].min(coord[1])]
    });

    let uv_scale = coords.chunks(2).fold([MAX, MAX], |result, coord| {
        [
            result[0].max(coord[0] - uv_offset[0]),
            result[1].max(coord[1] - uv_offset[1]),
        ]
    });

    (uv_offset, uv_scale)
}

pub fn calc_uv_offset_and_scale_inverse(coords: &[f32]) -> ([f32; 2], [f32; 2]) {
    let (uv_offset, uv_scale) = calc_uv_offset_and_scale(coords);
    let uv_scale_inverse = [rcp_safe(uv_scale[0]), rcp_safe(uv_scale[1])];
    (uv_offset, uv_scale_inverse)
}
