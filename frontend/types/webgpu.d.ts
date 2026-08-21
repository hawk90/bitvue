/**
 * Ambient WebGPU types (GPUDevice, GPUCanvasContext, navigator.gpu, ...).
 *
 * Referenced directly rather than via tsconfig's `types` array -- that array,
 * once set, disables automatic inclusion of every other ambient @types/*
 * package (e.g. @types/node), which would be a much bigger and unrelated
 * change than "add WebGPU types".
 */
/// <reference types="@webgpu/types" />
