use std::{ffi::CString, ops::Mul, ptr::null_mut};

use windows::{Win32::{Foundation::{BOOL, HWND, LPARAM, LRESULT, PSTR, WPARAM}, Graphics::{Direct3D::{*, Fxc::D3DCompileFromFile}, Direct3D11::*, Dxgi::{*, Common::*}}, System::LibraryLoader::GetModuleHandleA, UI::WindowsAndMessaging::*}, core::Interface};

mod data;
use data::*;
fn main() {
    unsafe  {WinMain();}
}

/*
#pragma comment(lib, "user32")
#pragma comment(lib, "d3d11")
#pragma comment(lib, "d3dcompiler")

///////////////////////////////////////////////////////////////////////////////////////////////////

#include <windows.h>
#include <d3d11_1.h>
#include <d3dcompiler.h>
#include <stdio.h>

#include <math.h> // sin, cos for rotation

#include "data.h" // example 3d model (the 'data.h' source file is provided below, along with 'shaders.hlsl')

///////////////////////////////////////////////////////////////////////////////////////////////////

#define TITLE "Minimal D3D11 by d7samurai"

///////////////////////////////////////////////////////////////////////////////////////////////////

struct float3 { float x, y, z; };
struct matrix { float m[4][4]; };

matrix operator*(matrix& m1, matrix& m2);
*/
///////////////////////////////////////////////////////////////////////////////////////////////////
struct float3 {  x: f32, y: f32, z: f32 }

impl float3 {
    fn new(x: f32, y: f32, z: f32) -> Self { Self { x, y, z }} 
}
struct matrix { m: [[f32; 4]; 4] }

impl matrix {
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
unsafe fn WinMain()
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

    let window = unsafe { 
        CreateWindowExA(Default::default(), "RustWindowClass", "Sample", WS_POPUP | WS_MAXIMIZE | WS_VISIBLE, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, None, None, None, null_mut())
    };

    ///////////////////////////////////////////////////////////////////////////////////////////////

    let feature_levels: D3D_FEATURE_LEVEL = D3D_FEATURE_LEVEL_11_0;
    
    let mut baseDevice = None;
    let mut baseDeviceContext = None;

    D3D11CreateDevice(None, D3D_DRIVER_TYPE_HARDWARE, None, D3D11_CREATE_DEVICE_BGRA_SUPPORT, &feature_levels, 1, D3D11_SDK_VERSION, &mut baseDevice, null_mut(), &mut baseDeviceContext);
    let baseDevice = baseDevice.unwrap();
    let baseDeviceContext = baseDeviceContext.unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////


    let mut device: ID3D11Device1 = baseDevice.cast().unwrap();
    let mut deviceContext: ID3D11DeviceContext1 = baseDeviceContext.cast().unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    let mut dxgiDevice: IDXGIDevice1 = device.cast().unwrap();

    let mut dxgiAdapter = dxgiDevice.GetAdapter().unwrap();

    let mut dxgiFactory: IDXGIFactory2 = dxgiAdapter.GetParent().unwrap();


    ///////////////////////////////////////////////////////////////////////////////////////////////



    let swapChainDesc = DXGI_SWAP_CHAIN_DESC1 {
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



    let swapChain = dxgiFactory.CreateSwapChainForHwnd(&device, window, &swapChainDesc, std::ptr::null(), None).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////



    let frameBuffer: ID3D11Texture2D = swapChain.GetBuffer(0).unwrap();


    let frameBufferView = device.CreateRenderTargetView(&frameBuffer, null_mut()).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    let mut depthBufferDesc = Default::default();

    frameBuffer.GetDesc(&mut depthBufferDesc); // base on framebuffer properties

    depthBufferDesc.Format    = DXGI_FORMAT_D24_UNORM_S8_UINT;
    depthBufferDesc.BindFlags = D3D11_BIND_DEPTH_STENCIL;



    let depthBuffer = device.CreateTexture2D(&depthBufferDesc, null_mut()).unwrap();
    let depthBufferView = device.CreateDepthStencilView(depthBuffer, null_mut()).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    let mut vsBlob = None;
    let mut errors = None;

    D3DCompileFromFile("shaders.hlsl", null_mut(), None, "vs_main", "vs_5_0", 0, 0, &mut vsBlob, &mut errors);
    if let Some(errors) = errors {
        println!("Failed to compile");
        panic!("{}", CString::from_raw(errors.GetBufferPointer() as *mut _ ).into_string().unwrap());
    }
    let vsBlob = vsBlob.unwrap();

    let vertexShader = device.CreateVertexShader(vsBlob.GetBufferPointer(), vsBlob.GetBufferSize(), None).unwrap();

    let inputElementDesc: [D3D11_INPUT_ELEMENT_DESC; 4]= [// float3 position, float3 normal, float2 texcoord, float3 color
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


    let inputLayout = device.CreateInputLayout(inputElementDesc.as_ptr(), inputElementDesc.len() as u32, vsBlob.GetBufferPointer(), vsBlob.GetBufferSize()).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    
    let mut psBlob = None;
    let mut errors = None;

    D3DCompileFromFile("shaders.hlsl", null_mut(), None, "ps_main", "ps_5_0", 0, 0, &mut psBlob, &mut errors);
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


    let rasterizerState = device.CreateRasterizerState1(&rasterizerDesc).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    let mut samplerDesc: D3D11_SAMPLER_DESC = Default::default();
    samplerDesc.Filter         = D3D11_FILTER_MIN_MAG_MIP_POINT;
    samplerDesc.AddressU       = D3D11_TEXTURE_ADDRESS_WRAP;
    samplerDesc.AddressV       = D3D11_TEXTURE_ADDRESS_WRAP;
    samplerDesc.AddressW       = D3D11_TEXTURE_ADDRESS_WRAP;
    samplerDesc.ComparisonFunc = D3D11_COMPARISON_NEVER;


    let samplerState = device.CreateSamplerState(&samplerDesc).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    let mut depthStencilDesc: D3D11_DEPTH_STENCIL_DESC  = Default::default();
    depthStencilDesc.DepthEnable    = BOOL(1);
    depthStencilDesc.DepthWriteMask = D3D11_DEPTH_WRITE_MASK_ALL;
    depthStencilDesc.DepthFunc      = D3D11_COMPARISON_LESS;

    let depthStencilState = device.CreateDepthStencilState(&depthStencilDesc).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    struct Constants
    {
        Transform: matrix,
        Projection: matrix,
        LightVector: float3,
    }

    let mut constantBufferDesc: D3D11_BUFFER_DESC = Default::default();
    constantBufferDesc.ByteWidth      = (std::mem::size_of::<Constants>() + 0xf & 0xfffffff0) as u32;
    constantBufferDesc.Usage          = D3D11_USAGE_DYNAMIC;
    constantBufferDesc.BindFlags      = D3D11_BIND_CONSTANT_BUFFER.0;
    constantBufferDesc.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE.0;

    let constantBuffer = device.CreateBuffer(&constantBufferDesc, null_mut()).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    let mut vertexBufferDesc: D3D11_BUFFER_DESC = Default::default();
    vertexBufferDesc.ByteWidth = std::mem::size_of_val(&VertexData) as u32;
    vertexBufferDesc.Usage     = D3D11_USAGE_IMMUTABLE;
    vertexBufferDesc.BindFlags = D3D11_BIND_VERTEX_BUFFER.0;

    let vertexData = D3D11_SUBRESOURCE_DATA{ pSysMem: VertexData.as_mut_ptr() as *mut _, ..Default::default() };


    let vertexBuffer = device.CreateBuffer(&vertexBufferDesc, &vertexData).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    let mut indexBufferDesc: D3D11_BUFFER_DESC = Default::default();
    indexBufferDesc.ByteWidth = std::mem::size_of_val(&IndexData) as u32;
    indexBufferDesc.Usage     = D3D11_USAGE_IMMUTABLE;
    indexBufferDesc.BindFlags = D3D11_BIND_INDEX_BUFFER.0;

    let indexData = D3D11_SUBRESOURCE_DATA{ pSysMem: IndexData.as_mut_ptr() as *mut _, ..Default::default() };

    let indexBuffer = device.CreateBuffer(&indexBufferDesc, &indexData).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    let mut textureDesc: D3D11_TEXTURE2D_DESC = Default::default();
    textureDesc.Width              = TEXTURE_WIDTH;  // in data.h
    textureDesc.Height             = TEXTURE_HEIGHT; // in data.h
    textureDesc.MipLevels          = 1;
    textureDesc.ArraySize          = 1;
    textureDesc.Format             = DXGI_FORMAT_R8G8B8A8_UNORM_SRGB;
    textureDesc.SampleDesc.Count   = 1;
    textureDesc.Usage              = D3D11_USAGE_IMMUTABLE;
    textureDesc.BindFlags          = D3D11_BIND_SHADER_RESOURCE;

    let mut textureData: D3D11_SUBRESOURCE_DATA = Default::default();
    textureData.pSysMem            = TextureData.as_mut_ptr() as *mut _;
    textureData.SysMemPitch        = TEXTURE_WIDTH * 4; // 4 bytes per pixel

    let texture = device.CreateTexture2D(&textureDesc, &textureData).unwrap();

    let textureView = device.CreateShaderResourceView(texture, null_mut()).unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    let w: f32 = depthBufferDesc.Width as f32;  // width
    let h: f32 = depthBufferDesc.Height as f32; // height
    let n: f32 = 1000.0;                                    // near
    let f: f32 = 1000000.0;                                 // far

    let mut modelRotation= float3    ::new(0.0, 0.0, 0.0 );
    let mut modelScale= float3       ::new( 400.0, 400.0, 400.0 );
    let mut modelTranslation= float3::new( 0.0, 0.0, 1500.0 );

    ///////////////////////////////////////////////////////////////////////////////////////////////

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

        let rotateX = matrix::new( 1., 0., 0., 0., 0., (cos(modelRotation.x)), -(sin(modelRotation.x)), 0., 0., (sin(modelRotation.x)), (cos(modelRotation.x)), 0., 0., 0., 0., 1. );
        let rotateY = matrix::new( (cos(modelRotation.y)), 0., (sin(modelRotation.y)), 0., 0., 1., 0., 0., -(sin(modelRotation.y)), 0., (cos(modelRotation.y)), 0., 0., 0., 0., 1. );
        let rotateZ   = matrix::new((cos(modelRotation.z)), -(sin(modelRotation.z)), 0., 0., (sin(modelRotation.z)), (cos(modelRotation.z)), 0., 0., 0., 0., 1., 0., 0., 0., 0., 1. );
        let scale     = matrix::new(modelScale.x, 0., 0., 0., 0., modelScale.y, 0., 0., 0., 0., modelScale.z, 0., 0., 0., 0., 1. );
        let translate = matrix::new( 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., modelTranslation.x, modelTranslation.y, modelTranslation.z, 1. );

        modelRotation.x += 0.005;
        modelRotation.y += 0.009;
        modelRotation.z += 0.001;

        ///////////////////////////////////////////////////////////////////////////////////////////



        let mappedSubresource = deviceContext.Map(&constantBuffer, 0, D3D11_MAP_WRITE_DISCARD, 0).unwrap();

        let constants = (mappedSubresource.pData as *mut Constants).as_mut().unwrap();

        constants.Transform   = rotateX * rotateY * rotateZ * scale * translate;
        constants.Projection  = matrix::new( 2. * n / w, 0., 0., 0., 0., 2. * n / h, 0., 0., 0., 0., f / (f - n), 1., 0., 0., n * f / (n - f), 0. );
        constants.LightVector = float3::new(1.0, -1.0, 1.0 );

        deviceContext.Unmap(&constantBuffer, 0);

        ///////////////////////////////////////////////////////////////////////////////////////////

        let backgroundColor: [f32; 4] = [ 0.025, 0.025, 0.025, 1.0];

        let stride = 11 * 4; // vertex size (11 floats: float3 position, float3 normal, float2 texcoord, float3 color)
        let offset = 0;

        let viewport = D3D11_VIEWPORT{ TopLeftX: 0.0, TopLeftY: 0.0, Width: w, Height: h, MinDepth: 0.0, MaxDepth: 1.0 };

        ///////////////////////////////////////////////////////////////////////////////////////////

        deviceContext.ClearRenderTargetView(&frameBufferView, backgroundColor.as_ptr());
        deviceContext.ClearDepthStencilView(&depthBufferView, D3D11_CLEAR_DEPTH.0 as u32, 1.0, 0);

        deviceContext.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        deviceContext.IASetInputLayout(&inputLayout);
        deviceContext.IASetVertexBuffers(0, 1, &Some(vertexBuffer.clone()), &stride, &offset);
        deviceContext.IASetIndexBuffer(&indexBuffer, DXGI_FORMAT_R32_UINT, 0);

        deviceContext.VSSetShader(&vertexShader, null_mut(), 0);
        deviceContext.VSSetConstantBuffers(0, 1, &Some(constantBuffer.clone()));

        deviceContext.RSSetViewports(1, &viewport);
        deviceContext.RSSetState(&rasterizerState);

        deviceContext.PSSetShader(&pixelShader, null_mut(), 0);
        deviceContext.PSSetShaderResources(0, 1, &Some(textureView.clone()));
        deviceContext.PSSetSamplers(0, 1, &Some(samplerState.clone()));

        deviceContext.OMSetRenderTargets(1, &Some(frameBufferView.clone()), &depthBufferView);
        deviceContext.OMSetDepthStencilState(&depthStencilState, 0);
        deviceContext.OMSetBlendState(None, null_mut(), 0xffffffff); // use default blend mode (i.e. disable)

        ///////////////////////////////////////////////////////////////////////////////////////////

        deviceContext.DrawIndexed(IndexData.len() as u32, 0, 0);

        ///////////////////////////////////////////////////////////////////////////////////////////

        swapChain.Present(1, 0);
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
impl Mul for matrix {
    type Output = matrix;
fn mul(self, m2: matrix) -> Self::Output
{
    let m1 = self;
    return matrix::new(
    
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
