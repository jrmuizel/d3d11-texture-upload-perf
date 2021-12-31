// Derived from https://gist.github.com/d7samurai/261c69490cce0620d0bfc93003cd1052

use std::{ffi::CString, ops::Mul, ptr::null_mut, time::Instant, collections::VecDeque};

use windows::{Win32::{Foundation::{BOOL, HWND, LPARAM, LRESULT, PSTR, RECT, WPARAM}, Graphics::{Direct3D::{*, Fxc::{D3DCompileFromFile, D3DCompile}}, Direct3D11::*, Dxgi::{*, Common::*}}, System::LibraryLoader::GetModuleHandleA, UI::WindowsAndMessaging::*}, core::Interface};

mod data;
use data::*;
fn main() {
    unsafe  {win_main();}
}

const SHADER_SOURCE: &[u8] = include_bytes!("shaders.hlsl");

struct Float3 {  x: f32, y: f32, z: f32 }

impl Float3 {
    fn new(x: f32, y: f32, z: f32) -> Self { Self { x, y, z }} 
}
struct Matrix { m: [[f32; 4]; 4] }

impl Matrix {
    fn new(
        a00: f32,
        a01: f32,
        a02: f32,
        a03: f32,
        a10: f32,
        a11: f32,
        a12: f32,
        a13: f32,
        a20: f32,
        a21: f32,
        a22: f32,
        a23: f32,
        a30: f32,
        a31: f32,
        a32: f32,
        a33: f32,
    ) -> Self {
        Self { m: [[a00, a01, a02, a03], [a10, a11, a12, a13],[a20, a21, a22, a23],[a30, a31, a32, a33]]}
    }
}

extern "system" fn wndproc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
         DefWindowProcA(window, message, wparam, lparam)
        
    }
}

//matrix operator*(matrix& m1, matrix& m2);
unsafe fn win_main()
{
    /*WNDCLASSEX wndClassEx = { sizeof(wndClassEx) };
    wndClassEx.lpfnWndProc   = DefWindowProcA;
    wndClassEx.lpszClassName = TITLE;
*/
    let instance = unsafe { GetModuleHandleA(None) };


    let wc = WNDCLASSEXA {
        cbSize: std::mem::size_of::<WNDCLASSEXA>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wndproc),
        hInstance: instance,
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) },
        lpszClassName: PSTR(b"RustWindowClass\0".as_ptr() as _),
        ..Default::default()
    };

    let atom = unsafe { RegisterClassExA(&wc) };
    assert_ne!(atom, 0);


    let size = (640, 480);
    let mut window_rect = RECT {
        left: 0,
        top: 0,
        right: size.0,
        bottom: size.1,
    };
    unsafe { AdjustWindowRect(&mut window_rect, WS_OVERLAPPEDWINDOW, false) };

    let window = unsafe { 
        CreateWindowExA(Default::default(), "RustWindowClass", "Sample", WS_OVERLAPPEDWINDOW, CW_USEDEFAULT, CW_USEDEFAULT,
        window_rect.right - window_rect.left,
        window_rect.bottom - window_rect.top,
          None, None, None, null_mut())
    };

    unsafe { ShowWindow(window, SW_SHOW) };


    ///////////////////////////////////////////////////////////////////////////////////////////////

    let feature_levels: D3D_FEATURE_LEVEL = D3D_FEATURE_LEVEL_10_0;
    
    let mut baseDevice = None;
    let mut baseDeviceContext = None;

    D3D11CreateDevice(None, D3D_DRIVER_TYPE_HARDWARE, None, D3D11_CREATE_DEVICE_BGRA_SUPPORT, &feature_levels, 1, D3D11_SDK_VERSION, &mut baseDevice, null_mut(), &mut baseDeviceContext).unwrap();
    let base_device = baseDevice.unwrap();
    let base_device_context = baseDeviceContext.unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////


    let device: ID3D11Device1 = base_device.cast().unwrap();
    let device_context: ID3D11DeviceContext1 = base_device_context.cast().unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    let dxgi_device: IDXGIDevice1 = device.cast().unwrap();

    let dxgi_adapter = dxgi_device.GetAdapter().unwrap();
    let desc = dxgi_adapter.GetDesc().unwrap();
    println!("Adapter: {:?}", String::from_utf16(&desc.Description.split(|x| *x == 0).next().unwrap()).unwrap());

    let dxgi_factory: IDXGIFactory2 = dxgi_adapter.GetParent().unwrap();


    ///////////////////////////////////////////////////////////////////////////////////////////////

    let swap_chain_desc = DXGI_SWAP_CHAIN_DESC1 {
        BufferCount: 2,
        Width: 0, // use window width
        Height: 0, // use window height
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            ..Default::default()
        },
        ..Default::default()
    };

    let swap_chain = dxgi_factory.CreateSwapChainForHwnd(&device, window, &swap_chain_desc, std::ptr::null(), None).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////



    let frame_buffer: ID3D11Texture2D = swap_chain.GetBuffer(0).unwrap();
    let frame_buffer_view = device.CreateRenderTargetView(&frame_buffer, null_mut()).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    let mut depth_buffer_desc = Default::default();
    frame_buffer.GetDesc(&mut depth_buffer_desc); // base on framebuffer properties
    depth_buffer_desc.Format    = DXGI_FORMAT_D24_UNORM_S8_UINT;
    depth_buffer_desc.BindFlags = D3D11_BIND_DEPTH_STENCIL;

    let depth_buffer = device.CreateTexture2D(&depth_buffer_desc, null_mut()).unwrap();
    let depth_buffer_view = device.CreateDepthStencilView(depth_buffer, null_mut()).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    let mut vs_blob = None;
    let mut errors = None;

    let ret = D3DCompile(SHADER_SOURCE.as_ptr() as _, SHADER_SOURCE.len(), "shaders.hlsl", null_mut(), None, "vs_main", "vs_4_0", 0, 0, &mut vs_blob, &mut errors);
    if let Some(errors) = errors {
        println!("Failed to compile");
        panic!("{}", CString::from_raw(errors.GetBufferPointer() as *mut _ ).into_string().unwrap());
    }
    ret.unwrap();
    let vs_blob = vs_blob.unwrap();

    let vertex_shader = device.CreateVertexShader(vs_blob.GetBufferPointer(), vs_blob.GetBufferSize(), None).unwrap();

    let input_element_desc: [D3D11_INPUT_ELEMENT_DESC; 4]= [// float3 position, float3 normal, float2 texcoord, float3 color
    D3D11_INPUT_ELEMENT_DESC {
        SemanticName: PSTR(b"POS\0".as_ptr() as _),
        SemanticIndex: 0,
        Format: DXGI_FORMAT_R32G32B32_FLOAT,
        InputSlot: 0,
        AlignedByteOffset: 0,
        InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
        InstanceDataStepRate: 0,
    },
    D3D11_INPUT_ELEMENT_DESC {
        SemanticName: PSTR(b"NOR\0".as_ptr() as _),
        SemanticIndex: 0,
        Format: DXGI_FORMAT_R32G32B32_FLOAT,
        InputSlot: 0,
        AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
        InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
        InstanceDataStepRate: 0,
    },
    D3D11_INPUT_ELEMENT_DESC {
        SemanticName: PSTR(b"TEX\0".as_ptr() as _),
        SemanticIndex: 0,
        Format: DXGI_FORMAT_R32G32_FLOAT,
        InputSlot: 0,
        AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
        InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
        InstanceDataStepRate: 0,
    },
    D3D11_INPUT_ELEMENT_DESC {
        SemanticName: PSTR(b"COL\0".as_ptr() as _),
        SemanticIndex: 0,
        Format: DXGI_FORMAT_R32G32B32_FLOAT,
        InputSlot: 0,
        AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
        InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
        InstanceDataStepRate: 0,
    },
    ];
    /*{
        { "POS", 0, DXGI_FORMAT_R32G32B32_FLOAT, 0,                            0, D3D11_INPUT_PER_VERTEX_DATA, 0 },
        { "NOR", 0, DXGI_FORMAT_R32G32B32_FLOAT, 0, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_INPUT_PER_VERTEX_DATA, 0 },
        { "TEX", 0, DXGI_FORMAT_R32G32_FLOAT,    0, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_INPUT_PER_VERTEX_DATA, 0 },
        { "COL", 0, DXGI_FORMAT_R32G32B32_FLOAT, 0, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_INPUT_PER_VERTEX_DATA, 0 },
    };*/


    let inputLayout = device.CreateInputLayout(input_element_desc.as_ptr(), input_element_desc.len() as u32, vs_blob.GetBufferPointer(), vs_blob.GetBufferSize()).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    
    let mut psBlob = None;
    let mut errors = None;

    D3DCompile(SHADER_SOURCE.as_ptr() as _, SHADER_SOURCE.len(), "shaders.hlsl", null_mut(), None, "ps_main", "ps_4_0", 0, 0, &mut psBlob, &mut errors);
    if let Some(errors) = errors {
        println!("Failed to compile");
        panic!("{}", CString::from_raw(errors.GetBufferPointer() as *mut _ ).into_string().unwrap());
    }
    let psBlob = psBlob.unwrap();

    let pixelShader = device.CreatePixelShader(psBlob.GetBufferPointer(), psBlob.GetBufferSize(), None).unwrap();


    ///////////////////////////////////////////////////////////////////////////////////////////////

    let mut rasterizerDesc: D3D11_RASTERIZER_DESC1 = Default::default();
    rasterizerDesc.FillMode = D3D11_FILL_SOLID;
    rasterizerDesc.CullMode = D3D11_CULL_BACK;

    let rasterizer_state = device.CreateRasterizerState1(&rasterizerDesc).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    let mut samplerDesc: D3D11_SAMPLER_DESC = Default::default();
    samplerDesc.Filter         = D3D11_FILTER_MIN_MAG_MIP_POINT;
    samplerDesc.AddressU       = D3D11_TEXTURE_ADDRESS_WRAP;
    samplerDesc.AddressV       = D3D11_TEXTURE_ADDRESS_WRAP;
    samplerDesc.AddressW       = D3D11_TEXTURE_ADDRESS_WRAP;
    samplerDesc.ComparisonFunc = D3D11_COMPARISON_NEVER;

    let sampler_state = device.CreateSamplerState(&samplerDesc).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    let mut depthStencilDesc: D3D11_DEPTH_STENCIL_DESC  = Default::default();
    depthStencilDesc.DepthEnable    = BOOL(1);
    depthStencilDesc.DepthWriteMask = D3D11_DEPTH_WRITE_MASK_ALL;
    depthStencilDesc.DepthFunc      = D3D11_COMPARISON_LESS;

    let depth_stencil_state = device.CreateDepthStencilState(&depthStencilDesc).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    struct Constants
    {
        Transform: Matrix,
        Projection: Matrix,
        LightVector: Float3,
    }

    let mut constant_buffer_desc: D3D11_BUFFER_DESC = Default::default();
    constant_buffer_desc.ByteWidth      = (std::mem::size_of::<Constants>() + 0xf & 0xfffffff0) as u32;
    constant_buffer_desc.Usage          = D3D11_USAGE_DYNAMIC;
    constant_buffer_desc.BindFlags      = D3D11_BIND_CONSTANT_BUFFER.0;
    constant_buffer_desc.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE.0;

    let constant_buffer = device.CreateBuffer(&constant_buffer_desc, null_mut()).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    let mut vertexBufferDesc: D3D11_BUFFER_DESC = Default::default();
    vertexBufferDesc.ByteWidth = std::mem::size_of_val(&VERTEX_DATA) as u32;
    vertexBufferDesc.Usage     = D3D11_USAGE_IMMUTABLE;
    vertexBufferDesc.BindFlags = D3D11_BIND_VERTEX_BUFFER.0;

    let vertex_data = D3D11_SUBRESOURCE_DATA{ pSysMem: VERTEX_DATA.as_ptr() as *mut _, ..Default::default() };

    let vertex_buffer = device.CreateBuffer(&vertexBufferDesc, &vertex_data).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    let mut index_buffer_desc: D3D11_BUFFER_DESC = Default::default();
    index_buffer_desc.ByteWidth = std::mem::size_of_val(&INDEX_DATA) as u32;
    index_buffer_desc.Usage     = D3D11_USAGE_IMMUTABLE;
    index_buffer_desc.BindFlags = D3D11_BIND_INDEX_BUFFER.0;

    let index_data = D3D11_SUBRESOURCE_DATA{ pSysMem: INDEX_DATA.as_ptr() as *mut _, ..Default::default() };

    let index_buffer = device.CreateBuffer(&index_buffer_desc, &index_data).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    let mut texture_desc: D3D11_TEXTURE2D_DESC = Default::default();
    let width = 2048;
    let height = 2048;
    texture_desc.Width              = width;  // in data.h
    texture_desc.Height             = height; // in data.h
    texture_desc.MipLevels          = 1;
    texture_desc.ArraySize          = 1;
    texture_desc.Format             = DXGI_FORMAT_R8G8B8A8_UNORM;
    texture_desc.SampleDesc.Count   = 1;
    texture_desc.Usage              = D3D11_USAGE_DEFAULT;
    texture_desc.BindFlags          = D3D11_BIND_SHADER_RESOURCE;

    let mut staging_desc: D3D11_TEXTURE2D_DESC = Default::default();
    staging_desc.Width              = width;  // in data.h
    staging_desc.Height             = height; // in data.h
    staging_desc.MipLevels          = 1;
    staging_desc.ArraySize          = 1;
    staging_desc.Format             = DXGI_FORMAT_R8G8B8A8_UNORM;
    staging_desc.SampleDesc.Count   = 1;
    staging_desc.Usage              = D3D11_USAGE_STAGING;
    staging_desc.BindFlags          = D3D11_BIND_FLAG(0);
    staging_desc.CPUAccessFlags     = D3D11_CPU_ACCESS_WRITE | D3D11_CPU_ACCESS_READ;

    let mut texture_data: D3D11_SUBRESOURCE_DATA = Default::default();

    let mut data = vec![0; (width*height*4) as usize];
    let mut rseed: u32 = 0;
    let mut rand = || -> u32
        {
            rseed = rseed.wrapping_mul(1103515245).wrapping_add(12345);
            rseed
        };
    for i in &mut data {
        *i = rand() as u8;
    }
    

    texture_data.pSysMem            = data.as_mut_ptr() as *mut _;
    texture_data.SysMemPitch        = width * 4; // 4 bytes per pixel
    let num_textures = 13; // 13 map, 11 nowait, 9 updatesubresource

    let wait = true;
    let update_subresource = true;

    let total_size = width * height * 4 * num_textures;
    let target_ms = 16.;
    println!("attempting to upload {}MB at {} GB/s", total_size as f32 / (1000.*1000.), (total_size as f32 / (1000.*1000. * 1000.)) / (target_ms * 1./(1000.)));
    println!("using {} textures of {}x{}", num_textures, width, height);
    let method = if update_subresource {
        "UpdateSubresource"
    } else {
        if wait {
            "Map"
        } else {
            "Map w/ D3D11_MAP_FLAG_DO_NOT_WAIT"
        }
    };
    println!("via {}", method);


    let mut textures = Vec::new();
    for _ in 0..num_textures {
        let texture = device.CreateTexture2D(&texture_desc, null_mut()).unwrap();
        let staging = device.CreateTexture2D(&staging_desc, null_mut()).unwrap();
        let staging2 = device.CreateTexture2D(&staging_desc, null_mut()).unwrap();

        let texture_view = device.CreateShaderResourceView(&texture, null_mut()).unwrap();

        textures.push((staging, texture, texture_view, staging2));
    }

    let w: f32 = depth_buffer_desc.Width as f32;  // width
    let h: f32 = depth_buffer_desc.Height as f32; // height
    let n: f32 = 1000.0;                                    // near
    let f: f32 = 1000000.0;                                 // far

    let mut model_rotation= Float3    ::new(0.0, 0.0, 0.0 );
    let model_scale= Float3       ::new( 400.0, 400.0, 400.0 );
    let model_translation= Float3::new( 0.0, 0.0, 1500.0 );

    ///////////////////////////////////////////////////////////////////////////////////////////////

    let mut last = Instant::now();
    let mut times = VecDeque::with_capacity(100);
    loop
    {
        let mut msg: MSG = Default::default();

        while PeekMessageA(&mut msg, None, 0, 0, PM_REMOVE).as_bool()
        {
            if msg.message == WM_KEYDOWN && msg.wParam == WPARAM(0x1B)/*VK_ESCAPE*/ { return  };
            DispatchMessageA(&msg);
        }

        fn cos(x: f32) -> f32 {
            x.cos()
        }
        fn sin(x: f32) -> f32 {
            x.sin()
        }
        ///////////////////////////////////////////////////////////////////////////////////////////

        let rotate_x = Matrix::new( 1., 0., 0., 0., 0., cos(model_rotation.x), -(sin(model_rotation.x)), 0., 0., sin(model_rotation.x), cos(model_rotation.x), 0., 0., 0., 0., 1. );
        let rotate_y = Matrix::new( cos(model_rotation.y), 0., sin(model_rotation.y), 0., 0., 1., 0., 0., -(sin(model_rotation.y)), 0., cos(model_rotation.y), 0., 0., 0., 0., 1. );
        let rotate_z   = Matrix::new(cos(model_rotation.z), -(sin(model_rotation.z)), 0., 0., sin(model_rotation.z), cos(model_rotation.z), 0., 0., 0., 0., 1., 0., 0., 0., 0., 1. );
        let scale     = Matrix::new(model_scale.x, 0., 0., 0., 0., model_scale.y, 0., 0., 0., 0., model_scale.z, 0., 0., 0., 0., 1. );
        let translate = Matrix::new( 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., model_translation.x, model_translation.y, model_translation.z, 1. );

        //model_rotation.x += 0.005;
        model_rotation.y += 0.0009;
        //model_rotation.z += 0.001;

        ///////////////////////////////////////////////////////////////////////////////////////////


        let mapped_subresource = device_context.Map(&constant_buffer, 0, D3D11_MAP_WRITE_DISCARD, 0).unwrap();

        let constants = (mapped_subresource.pData as *mut Constants).as_mut().unwrap();

        constants.Transform   = rotate_x * rotate_y * rotate_z * scale * translate;
        constants.Projection  = Matrix::new( 2. * n / w, 0., 0., 0., 0., 2. * n / h, 0., 0., 0., 0., f / (f - n), 1., 0., 0., n * f / (n - f), 0. );
        constants.LightVector = Float3::new(1.0, -1.0, 1.0 );

        device_context.Unmap(&constant_buffer, 0);

        ///////////////////////////////////////////////////////////////////////////////////////////

        let background_color: [f32; 4] = [ 0.025, 0.025, 0.025, 1.0];

        let stride = 11 * 4; // vertex size (11 floats: float3 position, float3 normal, float2 texcoord, float3 color)
        let offset = 0;

        let viewport = D3D11_VIEWPORT{ TopLeftX: 0.0, TopLeftY: 0.0, Width: w, Height: h, MinDepth: 0.0, MaxDepth: 1.0 };

        ///////////////////////////////////////////////////////////////////////////////////////////

        for texture in &textures {
            if update_subresource {
                device_context.UpdateSubresource(&texture.1, 0, null_mut(), data.as_mut_ptr() as *mut _, width * 4, 1);
            } else {
                let mapped_subresource = device_context.Map(&texture.0, 0, D3D11_MAP_READ_WRITE, if wait { 0 } else { D3D11_MAP_FLAG_DO_NOT_WAIT.0 as u32});
                let (src, mapped_subresource) = match mapped_subresource {
                    Ok(mapped_subresource) => (&texture.0, mapped_subresource),
                    _ => (&texture.3, device_context.Map(&texture.3, 0, D3D11_MAP_READ_WRITE, 0).unwrap())
                };
                std::ptr::copy_nonoverlapping(data.as_ptr(),mapped_subresource.pData as *mut _, data.len());
                device_context.Unmap(src, 0);
                device_context.CopySubresourceRegion(&texture.1, 0, 0, 0, 0, src, 0, null_mut());
            }

        }
        device_context.ClearRenderTargetView(&frame_buffer_view, background_color.as_ptr());
        device_context.ClearDepthStencilView(&depth_buffer_view, D3D11_CLEAR_DEPTH.0 as u32, 1.0, 0);

        device_context.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        device_context.IASetInputLayout(&inputLayout);
        device_context.IASetVertexBuffers(0, 1, &Some(vertex_buffer.clone()), &stride, &offset);
        device_context.IASetIndexBuffer(&index_buffer, DXGI_FORMAT_R32_UINT, 0);

        device_context.VSSetShader(&vertex_shader, null_mut(), 0);
        device_context.VSSetConstantBuffers(0, 1, &Some(constant_buffer.clone()));

        device_context.RSSetViewports(1, &viewport);
        device_context.RSSetState(&rasterizer_state);

        device_context.PSSetShader(&pixelShader, null_mut(), 0);
        //device_context.PSSetShaderResources(0, 1, &Some(texture_view.clone()));
        device_context.PSSetSamplers(0, 1, &Some(sampler_state.clone()));

        device_context.OMSetRenderTargets(1, &Some(frame_buffer_view.clone()), &depth_buffer_view);
        device_context.OMSetDepthStencilState(&depth_stencil_state, 0);
        device_context.OMSetBlendState(None, null_mut(), 0xffffffff); // use default blend mode (i.e. disable)

        ///////////////////////////////////////////////////////////////////////////////////////////

        device_context.DrawIndexed(INDEX_DATA.len() as u32, 0, 0);

        ///////////////////////////////////////////////////////////////////////////////////////////

        swap_chain.Present(1, 0);
        let now = Instant::now();
        let elapsed_seconds = (now - last).as_secs_f32();
        times.push_front(elapsed_seconds);
        let total: f32 = times.iter().sum();
        println!("Took {:.2}ms {:.2}GB/s avg {:.2} GB/s", elapsed_seconds * 1000., (total_size as f32 / (1000.*1000. * 1000.)) / elapsed_seconds, (total_size as f32 / (1000.*1000. * 1000.) * times.len() as f32) / total);
        if times.len() >= 100 {
            times.pop_back();
        }
        last = now;
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
impl Mul for Matrix {
    type Output = Matrix;
fn mul(self, m2: Matrix) -> Self::Output
{
    let m1 = self;
    return Matrix::new(
    
        m1.m[0][0] * m2.m[0][0] + m1.m[0][1] * m2.m[1][0] + m1.m[0][2] * m2.m[2][0] + m1.m[0][3] * m2.m[3][0],
        m1.m[0][0] * m2.m[0][1] + m1.m[0][1] * m2.m[1][1] + m1.m[0][2] * m2.m[2][1] + m1.m[0][3] * m2.m[3][1],
        m1.m[0][0] * m2.m[0][2] + m1.m[0][1] * m2.m[1][2] + m1.m[0][2] * m2.m[2][2] + m1.m[0][3] * m2.m[3][2],
        m1.m[0][0] * m2.m[0][3] + m1.m[0][1] * m2.m[1][3] + m1.m[0][2] * m2.m[2][3] + m1.m[0][3] * m2.m[3][3],
        m1.m[1][0] * m2.m[0][0] + m1.m[1][1] * m2.m[1][0] + m1.m[1][2] * m2.m[2][0] + m1.m[1][3] * m2.m[3][0],
        m1.m[1][0] * m2.m[0][1] + m1.m[1][1] * m2.m[1][1] + m1.m[1][2] * m2.m[2][1] + m1.m[1][3] * m2.m[3][1],
        m1.m[1][0] * m2.m[0][2] + m1.m[1][1] * m2.m[1][2] + m1.m[1][2] * m2.m[2][2] + m1.m[1][3] * m2.m[3][2],
        m1.m[1][0] * m2.m[0][3] + m1.m[1][1] * m2.m[1][3] + m1.m[1][2] * m2.m[2][3] + m1.m[1][3] * m2.m[3][3],
        m1.m[2][0] * m2.m[0][0] + m1.m[2][1] * m2.m[1][0] + m1.m[2][2] * m2.m[2][0] + m1.m[2][3] * m2.m[3][0],
        m1.m[2][0] * m2.m[0][1] + m1.m[2][1] * m2.m[1][1] + m1.m[2][2] * m2.m[2][1] + m1.m[2][3] * m2.m[3][1],
        m1.m[2][0] * m2.m[0][2] + m1.m[2][1] * m2.m[1][2] + m1.m[2][2] * m2.m[2][2] + m1.m[2][3] * m2.m[3][2],
        m1.m[2][0] * m2.m[0][3] + m1.m[2][1] * m2.m[1][3] + m1.m[2][2] * m2.m[2][3] + m1.m[2][3] * m2.m[3][3],
        m1.m[3][0] * m2.m[0][0] + m1.m[3][1] * m2.m[1][0] + m1.m[3][2] * m2.m[2][0] + m1.m[3][3] * m2.m[3][0],
        m1.m[3][0] * m2.m[0][1] + m1.m[3][1] * m2.m[1][1] + m1.m[3][2] * m2.m[2][1] + m1.m[3][3] * m2.m[3][1],
        m1.m[3][0] * m2.m[0][2] + m1.m[3][1] * m2.m[1][2] + m1.m[3][2] * m2.m[2][2] + m1.m[3][3] * m2.m[3][2],
        m1.m[3][0] * m2.m[0][3] + m1.m[3][1] * m2.m[1][3] + m1.m[3][2] * m2.m[2][3] + m1.m[3][3] * m2.m[3][3],
    );
}
}
