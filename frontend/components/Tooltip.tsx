/**
 * Tooltip Component
 *
 * Reusable tooltip for UI elements
 */

import { useState, useRef, useEffect, memo } from "react";
import React from "react";
import "./Tooltip.css";

interface TooltipProps {
  content: string;
  children: React.ReactElement;
  placement?: "top" | "bottom" | "left" | "right";
  delay?: number;
  disabled?: boolean;
}

export const Tooltip = memo(function Tooltip({
  content,
  children,
  placement = "top",
  delay = 300,
  disabled = false,
}: TooltipProps) {
  const [isVisible, setIsVisible] = useState(false);
  const timeoutRef = useRef<number | null>(null);
  const triggerRef = useRef<HTMLDivElement>(null);

  const showTooltip = () => {
    if (disabled) return;
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    timeoutRef.current = window.setTimeout(() => {
      setIsVisible(true);
    }, delay);
  };

  const hideTooltip = () => {
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    setIsVisible(false);
  };

  useEffect(() => {
    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, []);

  const childWithProps = React.cloneElement(children, {
    ref: triggerRef,
    onMouseEnter: showTooltip,
    onMouseLeave: hideTooltip,
    onFocus: showTooltip,
    onBlur: hideTooltip,
  });

  return (
    <div className="tooltip-container">
      {childWithProps}
      {isVisible && (
        <div className={`tooltip tooltip-${placement}`} role="tooltip">
          {content}
          <div className="tooltip-arrow" />
        </div>
      )}
    </div>
  );
});
