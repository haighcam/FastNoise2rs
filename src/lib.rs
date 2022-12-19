#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_void};

pub mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

lazy_static::lazy_static! {
    static ref NODEMETADATA: Vec<Metadata> = {
        let mut node_metadata = Vec::new();
        unsafe {
            for id in 0..ffi::fnGetMetadataCount() {
                let mut members = HashMap::new();

                for index in 0..ffi::fnGetMetadataVariableCount(id) {
                    let name = format_dimension_member(
                        &format_lookup(CStr::from_ptr(ffi::fnGetMetadataVariableName(id, index)).to_str().unwrap()),
                        ffi::fnGetMetadataVariableDimensionIdx(id, index)
                    );
                    let type_ = std::mem::transmute(ffi::fnGetMetadataVariableType(id, index));
                    let mut enum_names = HashMap::new();
                    if type_ == Type::Enum {
                        for eindex in 0..ffi::fnGetMetadataEnumCount(id, index) {
                            enum_names.insert(
                                format_lookup(CStr::from_ptr(ffi::fnGetMetadataEnumName(id, index, eindex)).to_str().unwrap()),
                                eindex
                            );
                        }
                    }
                    members.insert(
                        name.clone(),
                        Member { name, type_, index, enum_names }
                    );
                }
    
                for index in 0..ffi::fnGetMetadataNodeLookupCount(id) {
                    let name = format_dimension_member(
                        &format_lookup(CStr::from_ptr(ffi::fnGetMetadataNodeLookupName(id, index)).to_str().unwrap()),
                        ffi::fnGetMetadataNodeLookupDimensionIdx(id, index)
                    );
                    let type_ = Type::NodeLookup;
                    let enum_names = HashMap::new();
                    members.insert(
                        name.clone(),
                        Member { name, type_, index, enum_names }
                    );
                }
    
                for index in 0..ffi::fnGetMetadataHybridCount(id) {
                    let name = format_dimension_member(
                        &format_lookup(CStr::from_ptr(ffi::fnGetMetadataHybridName(id, index)).to_str().unwrap()),
                        ffi::fnGetMetadataHybridDimensionIdx(id, index)
                    );
                    let type_ = Type::Hybrid;
                    let enum_names = HashMap::new();
                    members.insert(
                        name.clone(),
                        Member { name, type_, index, enum_names }
                    );
                }
    
                let name = format_lookup(CStr::from_ptr(ffi::fnGetMetadataName(id)).to_str().unwrap());
                node_metadata.push(Metadata { id, name, members });
            }    
        }
        node_metadata
    };

    static ref METADATANAMELOOKUP: HashMap<String, i32> = {
        let mut node_metadata_lookup = HashMap::new();
        unsafe {
            for id in 0..ffi::fnGetMetadataCount() {
                let name = format_lookup(CStr::from_ptr(ffi::fnGetMetadataName(id)).to_str().unwrap());
                node_metadata_lookup.insert(name, id);
            }    
        }
        node_metadata_lookup
    };
}

#[derive(Debug)]
pub struct FastNoise {
    node_handle: *mut c_void,
    metadata_id: i32,
}

impl FastNoise {
    pub fn new(name: &str) -> Self {
        let metadata_id = *METADATANAMELOOKUP.get(&format_lookup(name)).unwrap();
        let node_handle = unsafe { ffi::fnNewFromMetadata(metadata_id, 0) };
        FastNoise { node_handle, metadata_id }
    }

    pub fn from_encoded_node_tree(encoded_node_tree: &str) -> Self {
        let encoded_node_tree = CString::new(encoded_node_tree).unwrap();
        unsafe { Self::from_handle(ffi::fnNewFromEncodedNodeTree(
            encoded_node_tree.as_ptr(),
            0
        )) }
    }

    fn from_handle(node_handle: *mut c_void) -> Self {
        let metadata_id = unsafe { ffi::fnGetMetadataID(node_handle) };
        FastNoise { node_handle, metadata_id }
    }

    pub fn get_simd_level(&self) -> u32 {
        unsafe { ffi::fnGetSIMDLevel(self.node_handle) }
    }

    pub fn set_float(&self, member: &str, val: f32) -> bool {
        let member = NODEMETADATA[self.metadata_id as usize].members.get(&format_lookup(member)).unwrap();
        match member.type_ {
            Type::Float => unsafe { ffi::fnSetVariableFloat(self.node_handle, member.index, val) },
            Type::Hybrid => unsafe { ffi::fnSetHybridFloat(self.node_handle, member.index, val) },
            _ => false
        }
    }

    pub fn with_float(self, member: &str, val: f32) -> Self {
        self.set_float(member, val);
        self
    }

    pub fn set_int(&self, member: &str, val: i32) -> bool {
        let member = NODEMETADATA[self.metadata_id as usize].members.get(&format_lookup(member)).unwrap();
        match member.type_ {
            Type::Int => unsafe { ffi::fnSetVariableIntEnum(self.node_handle, member.index, val) },
            _ => false
        }
    }

    pub fn with_int(self, member: &str, val: i32) -> Self {
        self.set_int(member, val);
        self
    }

    pub fn set_enum(&self, member: &str, val: &str) -> bool {
        let member = NODEMETADATA[self.metadata_id as usize].members.get(&format_lookup(member)).unwrap();
        match member.type_ {
            Type::Enum => unsafe { ffi::fnSetVariableIntEnum(self.node_handle, member.index, *member.enum_names.get(&format_lookup(val)).unwrap()) },
            _ => false
        }
    }

    pub fn with_enum(self, member: &str, val: &str) -> Self {
        self.set_enum(member, val);
        self
    }

    pub fn set_node(&self, member: &str, val: &Self) -> bool {
        let member = NODEMETADATA[self.metadata_id as usize].members.get(&format_lookup(member)).unwrap();
        match member.type_ {
            Type::NodeLookup => unsafe { ffi::fnSetNodeLookup(self.node_handle, member.index, val.node_handle) },
            Type::Hybrid => unsafe { ffi::fnSetHybridNodeLookup(self.node_handle, member.index, val.node_handle) },
            _ => false
        }
    }

    pub fn with_node(self, member: &str, val: &Self) -> Self {
        self.set_node(member, val);
        self
    }

    pub fn gen_uniform_grid_2d(&self, xstart: i32, ystart: i32, xsize: usize, ysize: usize, frequency: f32, seed: i32) -> (Vec<f32>, f32, f32) {
        let mut min_max = Vec::with_capacity(2);
        let mut vals = Vec::with_capacity(xsize*ysize);
        unsafe {
            min_max.set_len(2);
            vals.set_len(xsize*ysize);
            ffi::fnGenUniformGrid2D(self.node_handle, vals.as_mut_ptr(), xstart, ystart, xsize as i32, ysize as i32, frequency, seed, min_max.as_mut_ptr());
        }
        (vals, min_max[0], min_max[1])
    }

    pub fn gen_uniform_grid_3d(&self, xstart: i32, ystart: i32, zstart: i32, xsize: usize, ysize: usize, zsize: usize, frequency: f32, seed: i32) -> (Vec<f32>, f32, f32) {
        let mut min_max = Vec::with_capacity(2);
        let mut vals = Vec::with_capacity(xsize*ysize*zsize);
        unsafe {
            min_max.set_len(2);
            vals.set_len(xsize*ysize*zsize);
            ffi::fnGenUniformGrid3D(self.node_handle, vals.as_mut_ptr(), xstart, ystart, zstart, xsize as i32, ysize as i32, zsize as i32, frequency, seed, min_max.as_mut_ptr());
        }
        (vals, min_max[0], min_max[1])
    }

    pub fn gen_uniform_grid_4d(&self, xstart: i32, ystart: i32, zstart: i32, wstart: i32, xsize: usize, ysize: usize, zsize: usize, wsize: usize, frequency: f32, seed: i32) -> (Vec<f32>, f32, f32) {
        let mut min_max = Vec::with_capacity(2);
        let mut vals = Vec::with_capacity(xsize*ysize*zsize*wsize);
        unsafe {
            min_max.set_len(2);
            vals.set_len(xsize*ysize*zsize*wsize);
            ffi::fnGenUniformGrid4D(self.node_handle, vals.as_mut_ptr(), xstart, ystart, zstart, wstart, xsize as i32, ysize as i32, zsize as i32, wsize as i32, frequency, seed, min_max.as_mut_ptr());
        }
        (vals, min_max[0], min_max[1])
    }

    pub fn gen_tileble_grid_2d(&self, xsize: usize, ysize: usize, frequency: f32, seed: i32) -> (Vec<f32>, f32, f32) {
        let mut min_max = Vec::with_capacity(2);
        let mut vals = Vec::with_capacity(xsize*ysize);
        unsafe {
            min_max.set_len(2);
            vals.set_len(xsize*ysize);
            ffi::fnGenTileable2D(self.node_handle, vals.as_mut_ptr(), xsize as i32, ysize as i32, frequency, seed, min_max.as_mut_ptr());
        }
        (vals, min_max[0], min_max[1])
    }

    pub fn gen_position_array_2d(&self, xpos: &[f32], ypos: &[f32], xoffset: f32, yoffset: f32, seed: i32) -> (Vec<f32>, f32, f32) {
        let mut min_max = Vec::with_capacity(2);
        let mut vals = Vec::with_capacity(xpos.len());
        unsafe {
            min_max.set_len(2);
            vals.set_len(xpos.len());
            ffi::fnGenPositionArray2D(self.node_handle, vals.as_mut_ptr(), xpos.len() as i32, xpos.as_ptr(), ypos.as_ptr(), xoffset, yoffset, seed, min_max.as_mut_ptr());
        }
        (vals, min_max[0], min_max[1])
    }

    pub fn gen_position_array_3d(&self, xpos: &[f32], ypos: &[f32], zpos: &[f32], xoffset: f32, yoffset: f32, zoffset: f32, seed: i32) -> (Vec<f32>, f32, f32) {
        let mut min_max = Vec::with_capacity(2);
        let mut vals = Vec::with_capacity(xpos.len());
        unsafe {
            min_max.set_len(2);
            vals.set_len(xpos.len());
            ffi::fnGenPositionArray3D(self.node_handle, vals.as_mut_ptr(), xpos.len() as i32, xpos.as_ptr(), ypos.as_ptr(), zpos.as_ptr(), xoffset, yoffset, zoffset, seed, min_max.as_mut_ptr());
        }
        (vals, min_max[0], min_max[1])
    }


    pub fn gen_position_array_4d(&self, xpos: &[f32], ypos: &[f32], zpos: &[f32], wpos: &[f32], xoffset: f32, yoffset: f32, zoffset: f32, woffset: f32, seed: i32) -> (Vec<f32>, f32, f32) {
        let mut min_max = Vec::with_capacity(2);
        let mut vals = Vec::with_capacity(xpos.len());
        unsafe {
            min_max.set_len(2);
            vals.set_len(xpos.len());
            ffi::fnGenPositionArray4D(self.node_handle, vals.as_mut_ptr(), xpos.len() as i32, xpos.as_ptr(), ypos.as_ptr(), zpos.as_ptr(), wpos.as_ptr(), xoffset, yoffset, zoffset, woffset, seed, min_max.as_mut_ptr());
        }
        (vals, min_max[0], min_max[1])
    }

    pub fn gen_single_2d(&self, x: f32, y:f32, seed: i32) -> f32 {
        unsafe { ffi::fnGenSingle2D(self.node_handle, x, y, seed) }
    }

    pub fn gen_single_3d(&self, x: f32, y:f32, z: f32, seed: i32) -> f32 {
        unsafe { ffi::fnGenSingle3D(self.node_handle, x, y, z, seed) }
    }

    pub fn gen_single_4d(&self, x: f32, y:f32, z: f32, w:f32, seed: i32) -> f32 {
        unsafe { ffi::fnGenSingle4D(self.node_handle, x, y, z, w, seed) }
    }
}

impl Drop for FastNoise {
    fn drop(&mut self) {
        unsafe {
            ffi::fnDeleteNodeRef(self.node_handle)
        }
    }
}

#[repr(i32)]
#[derive(PartialEq, Debug)]
enum Type {
    Float,
    Int,
    Enum,
    NodeLookup,
    Hybrid,
}

#[derive(Debug)]
struct Member {
    name: String,
    type_: Type,
    index: i32,
    enum_names: HashMap<String, i32>,
}

#[derive(Debug)]
struct Metadata {
    id: i32,
    name: String,
    members: HashMap<String, Member>,
}

fn format_lookup(s: &str) -> String {
    s.replace(" ", "").to_lowercase()
}

fn format_dimension_member(s: &str, dim: i32) -> String {
    match dim {
        1 => s.to_string() + "x",
        2 => s.to_string() + "y",
        3 => s.to_string() + "z",
        4 => s.to_string() + "w",
        _ => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fucntionality_test() {
        // make sure things don't error, not a very robust test

        let max_smooth = FastNoise::new("MaxSmooth")
        .with_node("LHS", &FastNoise::new("FractalFbm")
            .with_node("Source", &FastNoise::new("Simplex"))
            .with_float("Gain", 0.3)
            .with_float("Lacunarity", 0.6)
        )
        .with_node("RHS", &FastNoise::new("AddDimension")
            .with_node("Source", &FastNoise::new("CellularDistance")
                .with_enum("ReturnType", "Index0Add1")
                .with_int("DistanceIndex0", 2)
            )
            .with_float("NewDimensionPosition", 0.5)
        );

        let simd_level = max_smooth.get_simd_level();

        max_smooth.gen_single_3d(1.0, 1.0, 1.0, 1337);
    }
}