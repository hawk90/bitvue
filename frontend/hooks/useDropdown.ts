/**
 * useDropdown Hook
 *
 * Custom hook for managing dropdown state
 * Provides consistent behavior for dropdown menus across the application
 */

import { useState, useCallback, useRef, useEffect } from "react";

export interface UseDropdownOptions {
  /** Initial open state */
  initialOpen?: boolean;
  /** Callback when dropdown opens */
  onOpen?: () => void;
  /** Callback when dropdown closes */
  onClose?: () => void;
  /** Whether to close on click outside */
  closeOnClickOutside?: boolean;
  /** Whether to close on escape key */
  closeOnEscape?: boolean;
}

export interface UseDropdownReturn {
  /** Whether the dropdown is currently open */
  isOpen: boolean;
  /** Open the dropdown */
  open: () => void;
  /** Close the dropdown */
  close: () => void;
  /** Toggle the dropdown open/close */
  toggle: () => void;
  /** Ref to attach to the dropdown container element */
  dropdownRef: React.RefObject<HTMLDivElement>;
  /** Ref to attach to the trigger element */
  triggerRef: React.RefObject<HTMLElement>;
}

/**
 * Hook for managing dropdown state with common behaviors
 *
 * @example
 * ```tsx
 * const { isOpen, toggle, close, dropdownRef, triggerRef } = useDropdown();
 *
 * return (
 *   <>
 *     <button ref={triggerRef} onClick={toggle}>Toggle</button>
 *     {isOpen && (
 *       <div ref={dropdownRef}>
 *         <DropdownItems />
 *       </div>
 *     )}
 *   </>
 * );
 * ```
 */
export function useDropdown(
  options: UseDropdownOptions = {},
): UseDropdownReturn {
  const {
    initialOpen = false,
    onOpen,
    onClose,
    closeOnClickOutside = true,
    closeOnEscape = true,
  } = options;

  const [isOpen, setIsOpen] = useState(initialOpen);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLElement>(null);

  const open = useCallback(() => {
    if (!isOpen) {
      setIsOpen(true);
      onOpen?.();
    }
  }, [isOpen, onOpen]);

  const close = useCallback(() => {
    if (isOpen) {
      setIsOpen(false);
      onClose?.();
    }
  }, [isOpen, onClose]);

  const toggle = useCallback(() => {
    setIsOpen((prev) => {
      if (!prev) {
        onOpen?.();
      } else {
        onClose?.();
      }
      return !prev;
    });
  }, [onOpen, onClose]);

  // Handle click outside
  useEffect(() => {
    if (!isOpen || !closeOnClickOutside) return;

    const handleClickOutside = (event: MouseEvent) => {
      const target = event.target as Node;
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(target) &&
        triggerRef.current &&
        !triggerRef.current.contains(target)
      ) {
        close();
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [isOpen, closeOnClickOutside, close]);

  // Handle escape key
  useEffect(() => {
    if (!isOpen || !closeOnEscape) return;

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        close();
      }
    };

    document.addEventListener("keydown", handleEscape);
    return () => document.removeEventListener("keydown", handleEscape);
  }, [isOpen, closeOnEscape, close]);

  // Focus trap when open
  useEffect(() => {
    if (!isOpen || !dropdownRef.current) return;

    const focusableElements = dropdownRef.current.querySelectorAll<
      HTMLElement | SVGElement
    >(
      'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])',
    );

    if (focusableElements.length > 0) {
      focusableElements[0].focus();
    }
  }, [isOpen]);

  return {
    isOpen,
    open,
    close,
    toggle,
    dropdownRef,
    triggerRef,
  };
}

/**
 * Hook for managing dropdown with controlled open state
 * Use this when you need external control over the dropdown state
 */
export function useControlledDropdown(
  isOpen: boolean,
  onClose: () => void,
  options: Pick<
    UseDropdownOptions,
    "closeOnClickOutside" | "closeOnEscape"
  > = {},
): UseDropdownReturn {
  const { closeOnClickOutside = true, closeOnEscape = true } = options;
  const dropdownRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLElement>(null);

  const close = useCallback(() => {
    onClose();
  }, [onClose]);

  // Handle click outside
  useEffect(() => {
    if (!isOpen || !closeOnClickOutside) return;

    const handleClickOutside = (event: MouseEvent) => {
      const target = event.target as Node;
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(target) &&
        triggerRef.current &&
        !triggerRef.current.contains(target)
      ) {
        close();
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [isOpen, closeOnClickOutside, close]);

  // Handle escape key
  useEffect(() => {
    if (!isOpen || !closeOnEscape) return;

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        close();
      }
    };

    document.addEventListener("keydown", handleEscape);
    return () => document.removeEventListener("keydown", handleEscape);
  }, [isOpen, closeOnEscape, close]);

  return {
    isOpen,
    open: () => {}, // No-op for controlled
    close,
    toggle: () => {}, // No-op for controlled
    dropdownRef,
    triggerRef,
  };
}
