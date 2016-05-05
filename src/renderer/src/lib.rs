#![crate_name = "amethyst_renderer"]
#![crate_type = "lib"]
#![doc(html_logo_url = "http://tinyurl.com/hgsb45k")]
#![allow(dead_code)]

//! High-level rendering engine with multiple backends.

#[macro_use]
extern crate gfx;
#[macro_use]
extern crate mopa;

extern crate glutin;
extern crate cgmath;

pub mod target;
pub mod pass;

use std::any::TypeId;
use std::collections::HashMap;

pub use pass::PassDescription;
pub use target::Target;
pub use pass::Pass;

pub struct Renderer<R: gfx::Resources, C: gfx::CommandBuffer<R>> {
    command_buffer: gfx::Encoder<R, C>,
    passes: HashMap<(TypeId, TypeId), Box<Fn(&Box<PassDescription>, &Target, &Frame<R>, &mut gfx::Encoder<R, C>)>>
}

// placeholder
gfx_vertex_struct!( VertexPosNormal {
    pos: [f32; 3] = "a_Pos",
    normal: [f32; 3] = "a_Normal",
});

impl<R, C> Renderer<R, C>
    where R: gfx::Resources,
          C: gfx::CommandBuffer<R>
{
    /// Create a new Render pipline
    pub fn new(combuf: C) -> Renderer<R, C> {
        Renderer {
            command_buffer: combuf.into(),
            passes: HashMap::new()
        }
    }

    /// Load all known passes
    pub fn load_all<F>(&mut self, factory: &mut F)
        where F: gfx::Factory<R>
    {
        self.add_pass(pass::forward::Clear);
        self.add_pass(pass::forward::DrawNoShading::new(factory));
        self.add_pass(pass::forward::Wireframe::new(factory));

        self.add_pass(pass::deferred::Clear);
        self.add_pass(pass::deferred::DrawPass::new(factory));
        self.add_pass(pass::deferred::BlitLayer::new(factory));
        self.add_pass(pass::deferred::LightingPass::new(factory));
    }

    /// Add a pass to the table of available passes
    pub fn add_pass<A, T, P>(&mut self, p: P)
        where P: Pass<R, Arg=A, Target=T> + 'static,
              A: PassDescription,
              T: Target
    {
        let id = (TypeId::of::<A>(), TypeId::of::<T>());
        self.passes.insert(id, Box::new(move |a: &Box<PassDescription>, t: &Target, frame: &Frame<R>, encoder: &mut gfx::Encoder<R, C>| {
            let a = a.downcast_ref::<A>().unwrap();
            let t = t.downcast_ref::<T>().unwrap();
            p.apply(a, t, frame, encoder)
        }));
    }

    /// Execute all passes
    pub fn submit<D>(&mut self, frame: &Frame<R>, device: &mut D)
        where D: gfx::Device<Resources=R, CommandBuffer=C>
    {
        for layer in &frame.layers {
            let fb = frame.targets.get(&layer.target).unwrap();
            for desc in &layer.passes {
                let id = (mopa::Any::get_type_id(&**desc), mopa::Any::get_type_id(&**fb));
                if let Some(pass)= self.passes.get(&id) {
                    pass(desc, &**fb, &frame, &mut self.command_buffer);
                } else{
                    panic!("No pass implementation found for target={}, pass={:?}", layer.target, desc);
                }
            }
        }
        self.command_buffer.flush(device);
        device.cleanup();
    }
}

pub struct Fragment<R: gfx::Resources> {
    pub transform: [[f32; 4]; 4],
    pub buffer: gfx::handle::Buffer<R, VertexPosNormal>,
    pub slice: gfx::Slice<R>,
    /// ambient colour
    pub ka: [f32; 4],
    /// diffuse colour
    pub kd: [f32; 4]
}

// placeholder light
pub struct Light {
    // clip scale
    pub center: [f32; 3],
    pub radius: f32,

    pub color: [f32; 4],
    // color * (pc + pl / r + pc / (r^2))
    pub propagation_constant: f32,
    pub propagation_linear: f32,
    pub propagation_r_square: f32,
}

pub struct Scene<R: gfx::Resources> {
    pub fragments: Vec<Fragment<R>>,
    pub lights: Vec<Light>
}

impl<R: gfx::Resources> Scene<R> {
    /// Create an empty scene
    pub fn new() -> Scene<R> {
        Scene{
            fragments: vec![],
            lights: vec![]
        }
    }
}

#[derive(Copy, Clone)]
pub struct Camera {
    pub projection: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
}

pub struct Layer {
    pub target: String,
    pub passes: Vec<Box<PassDescription>>,
}

impl Layer {
    pub fn new<A>(target: A, passes: Vec<Box<PassDescription>>) -> Layer
        where String: From<A>
    {
        Layer {
            target: String::from(target),
            passes: passes
        }
    }
}

/// The render job submission
pub struct Frame<R: gfx::Resources> {
    pub layers: Vec<Layer>,
    pub targets: HashMap<String, Box<Target>>,
    pub scenes: HashMap<String, Scene<R>>,
    pub cameras: HashMap<String, Camera>
}

impl<R: gfx::Resources> Frame<R> {
    /// Create an empty Frame
    pub fn new() -> Frame<R> {
        Frame {
            layers: vec![],
            targets: HashMap::new(),
            scenes: HashMap::new(),
            cameras: HashMap::new()
        }
    }
}