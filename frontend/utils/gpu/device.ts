/**
 * WebGPU device management
 *
 * One shared GPUDevice per app session -- devices are expensive to request and
 * this app only ever needs one (a single video-display surface at a time).
 */

import { createLogger } from "../logger";

const logger = createLogger("gpu/device");

let cachedDevice: GPUDevice | null = null;
let requestInFlight: Promise<GPUDevice | null> | null = null;

/** Synchronous capability check -- does NOT confirm an adapter/device is actually obtainable. */
export function isWebGPUApiPresent(): boolean {
  return typeof navigator !== "undefined" && "gpu" in navigator;
}

async function requestDevice(): Promise<GPUDevice | null> {
  if (!isWebGPUApiPresent()) return null;
  try {
    const adapter = await navigator.gpu.requestAdapter();
    if (!adapter) {
      logger.warn("navigator.gpu.requestAdapter() returned null");
      return null;
    }
    const device = await adapter.requestDevice();
    device.lost.then((info) => {
      logger.warn("GPUDevice lost", info.message);
      if (cachedDevice === device) {
        cachedDevice = null;
      }
    });
    return device;
  } catch (error) {
    logger.warn("Failed to acquire WebGPU device", error);
    return null;
  }
}

/** Returns the shared GPUDevice, requesting it once and caching it for the session. */
export async function getSharedGpuDevice(): Promise<GPUDevice | null> {
  if (cachedDevice) return cachedDevice;
  if (!requestInFlight) {
    requestInFlight = requestDevice().then((device) => {
      cachedDevice = device;
      requestInFlight = null;
      return device;
    });
  }
  return requestInFlight;
}

/** Resolves once, used by the fallback decision in VideoCanvas -- true only if a device was actually obtained. */
export async function isWebGPUAvailable(): Promise<boolean> {
  const device = await getSharedGpuDevice();
  return device !== null;
}
