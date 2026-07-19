import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function prefersReducedMotion(): boolean {
  if (typeof window === 'undefined') return false
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches
}

// Only "macos"/"windows" ever ship (see tauri.conf.json's bundle.targets:
// app/dmg/nsis, no linux target) -- Rust-sourced (SystemState.platform), not
// navigator sniffing, since this app already has an authoritative answer.
export function isMacPlatform(platform: string): boolean {
  return platform !== 'windows'
}

// event.metaKey is the Cmd key on macOS but the Windows/Super key elsewhere,
// which OS-level shortcuts already claim -- Windows/Linux users need ctrlKey
// for an app shortcut to actually reach them.
export function modKeyPressed(event: KeyboardEvent, platform: string): boolean {
  return isMacPlatform(platform) ? event.metaKey : event.ctrlKey
}

export type WithoutChild<T> = T extends { child?: any } ? Omit<T, "child"> : T;

export type WithoutChildren<T> = T extends { children?: any } ? Omit<T, "children"> : T;
export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;
export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & { ref?: U | null };
