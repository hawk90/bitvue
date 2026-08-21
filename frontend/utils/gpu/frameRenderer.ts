/**
 * WebGPU-backed YUV renderer.
 *
 * The WebGPU analog of ../yuv/renderer.ts's YUVRenderer -- same public shape
 * (attachCanvas/render/clear) so VideoCanvas.tsx can pick one or the other
 * without its logic forking heavily. Uploads Y/U/V planes as textures each
 * frame and runs frameShader.ts's fragment shader into the WebGPU canvas
 * context; unlike the CPU path there's no separate offscreen+drawImage scale
 * step -- the fullscreen-triangle vertex stage covers the destination canvas
 * (already sized for devicePixelRatio by the caller) directly.
 */
import type { YUVFrame } from "../../types/yuv";
import { Colorspace, COLORSPACE_MATRICES } from "../../types/yuv";
import { FRAME_SHADER_WGSL } from "./frameShader";
import { getSharedGpuDevice } from "./device";
import { createLogger } from "../logger";

const logger = createLogger("gpu/frameRenderer");

interface PlaneTexture {
  texture: GPUTexture;
  width: number;
  height: number;
}

/** Derives a plane's texture height from its byte length and row stride -- this
 * codebase's wire format always sets stride == active plane width (no row
 * padding), so this recovers the row count exactly. */
function planeHeight(byteLength: number, stride: number): number {
  return stride > 0 ? Math.max(1, Math.round(byteLength / stride)) : 1;
}

export class WebGpuFrameRenderer {
  private context: GPUCanvasContext | null = null;
  private device: GPUDevice | null = null;
  private format: GPUTextureFormat = "bgra8unorm";
  private pipeline: GPURenderPipeline | null = null;
  private sampler: GPUSampler | null = null;
  private uniformBuffer: GPUBuffer | null = null;
  private yPlane: PlaneTexture | null = null;
  private uPlane: PlaneTexture | null = null;
  private vPlane: PlaneTexture | null = null;
  private ready = false;

  constructor(canvas?: HTMLCanvasElement) {
    if (canvas) {
      // Fire-and-forget from the constructor to match YUVRenderer's synchronous
      // constructor shape; callers that need to know readiness before their
      // first render() should call attachCanvas() directly and await it.
      void this.attachCanvas(canvas);
    }
  }

  /** Requests the shared GPU device (if not already cached) and configures the canvas. */
  async attachCanvas(canvas: HTMLCanvasElement): Promise<boolean> {
    const device = await getSharedGpuDevice();
    if (!device) return false;

    const context = canvas.getContext("webgpu") as GPUCanvasContext | null;
    if (!context) {
      logger.warn("canvas.getContext('webgpu') returned null");
      return false;
    }

    this.device = device;
    this.context = context;
    this.format = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: this.format, alphaMode: "opaque" });

    this.buildPipeline();
    this.ready = true;
    return true;
  }

  isReady(): boolean {
    return this.ready;
  }

  private buildPipeline(): void {
    if (!this.device) return;
    const module = this.device.createShaderModule({ code: FRAME_SHADER_WGSL });

    this.sampler = this.device.createSampler({
      magFilter: "nearest",
      minFilter: "nearest",
    });

    // vec4f-aligned uniform: rv, gu, gv, bu.
    this.uniformBuffer = this.device.createBuffer({
      size: 16,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });

    this.pipeline = this.device.createRenderPipeline({
      layout: "auto",
      vertex: { module, entryPoint: "vs_main" },
      fragment: {
        module,
        entryPoint: "fs_main",
        targets: [{ format: this.format }],
      },
      primitive: { topology: "triangle-list" },
    });
  }

  private uploadPlane(
    existing: PlaneTexture | null,
    bytes: Uint8Array,
    width: number,
    height: number,
  ): PlaneTexture {
    if (!this.device) {
      throw new Error(
        "WebGpuFrameRenderer.uploadPlane called before attachCanvas resolved",
      );
    }
    let plane = existing;
    if (!plane || plane.width !== width || plane.height !== height) {
      plane?.texture.destroy();
      plane = {
        texture: this.device.createTexture({
          size: { width, height },
          format: "r8unorm",
          usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
        }),
        width,
        height,
      };
    }
    // writeTexture (unlike copyBufferToTexture) has no 256-byte bytesPerRow
    // alignment requirement, so the raw plane stride can be used directly.
    // The cast works around a lib.dom.d.ts/@webgpu/types generic-parameter
    // mismatch (Uint8Array<ArrayBufferLike> vs ArrayBufferView<ArrayBuffer>),
    // not a real runtime concern -- these views are never backed by SharedArrayBuffer.
    this.device.queue.writeTexture(
      { texture: plane.texture },
      bytes as unknown as BufferSource,
      { bytesPerRow: width },
      { width, height },
    );
    return plane;
  }

  /** Renders one YUV frame. No-op if attachCanvas hasn't resolved successfully yet. */
  render(frame: YUVFrame, colorspace: Colorspace = Colorspace.BT709): void {
    if (
      !this.ready ||
      !this.device ||
      !this.context ||
      !this.pipeline ||
      !this.sampler ||
      !this.uniformBuffer
    ) {
      return;
    }

    this.yPlane = this.uploadPlane(
      this.yPlane,
      frame.y,
      frame.yStride,
      frame.height,
    );
    this.uPlane = this.uploadPlane(
      this.uPlane,
      frame.u,
      frame.uStride,
      planeHeight(frame.u.length, frame.uStride),
    );
    this.vPlane = this.uploadPlane(
      this.vPlane,
      frame.v,
      frame.vStride,
      planeHeight(frame.v.length, frame.vStride),
    );

    const matrix = COLORSPACE_MATRICES[colorspace];
    this.device.queue.writeBuffer(
      this.uniformBuffer,
      0,
      new Float32Array([matrix.rv, matrix.gu, matrix.gv, matrix.bu]),
    );

    const bindGroup = this.device.createBindGroup({
      layout: this.pipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: this.sampler },
        { binding: 1, resource: this.yPlane.texture.createView() },
        { binding: 2, resource: this.uPlane.texture.createView() },
        { binding: 3, resource: this.vPlane.texture.createView() },
        { binding: 4, resource: { buffer: this.uniformBuffer } },
      ],
    });

    const encoder = this.device.createCommandEncoder();
    const pass = encoder.beginRenderPass({
      colorAttachments: [
        {
          view: this.context.getCurrentTexture().createView(),
          clearValue: { r: 0, g: 0, b: 0, a: 1 },
          loadOp: "clear",
          storeOp: "store",
        },
      ],
    });
    pass.setPipeline(this.pipeline);
    pass.setBindGroup(0, bindGroup);
    pass.draw(3);
    pass.end();
    this.device.queue.submit([encoder.finish()]);
  }

  /** Clears the canvas to black -- mirrors YUVRenderer.clear()'s contract. */
  clear(): void {
    if (!this.device || !this.context) return;
    const encoder = this.device.createCommandEncoder();
    const pass = encoder.beginRenderPass({
      colorAttachments: [
        {
          view: this.context.getCurrentTexture().createView(),
          clearValue: { r: 0, g: 0, b: 0, a: 1 },
          loadOp: "clear",
          storeOp: "store",
        },
      ],
    });
    pass.end();
    this.device.queue.submit([encoder.finish()]);
  }

  /** Releases GPU texture resources. Does not destroy the shared device (session-scoped). */
  destroy(): void {
    this.yPlane?.texture.destroy();
    this.uPlane?.texture.destroy();
    this.vPlane?.texture.destroy();
    this.yPlane = null;
    this.uPlane = null;
    this.vPlane = null;
    this.ready = false;
  }
}
