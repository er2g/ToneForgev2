import { useState, useRef, useEffect, ReactNode } from "react";
import "./Tooltip.css";

interface TooltipProps {
  content: string;
  children: ReactNode;
  position?: "top" | "bottom" | "left" | "right";
  delay?: number;
}

export function Tooltip({
  content,
  children,
  position = "top",
  delay = 300,
}: TooltipProps) {
  const [visible, setVisible] = useState(false);
  const timeoutRef = useRef<number | null>(null);

  const showTooltip = () => {
    timeoutRef.current = window.setTimeout(() => {
      setVisible(true);
    }, delay);
  };

  const hideTooltip = () => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    setVisible(false);
  };

  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, []);

  return (
    <div
      className="tooltip-wrapper"
      onMouseEnter={showTooltip}
      onMouseLeave={hideTooltip}
      onFocus={showTooltip}
      onBlur={hideTooltip}
    >
      {children}
      {visible && content && (
        <div className={`tooltip tooltip-${position}`}>
          {content}
          <div className="tooltip-arrow" />
        </div>
      )}
    </div>
  );
}

// Parameter tooltip with more info
interface ParamTooltipProps {
  name: string;
  value: string;
  unit?: string;
  hint?: string;
  children: ReactNode;
}

export function ParamTooltip({
  name,
  value,
  unit,
  hint,
  children,
}: ParamTooltipProps) {
  const content = `${name}: ${value}${unit ? ` ${unit}` : ""}${hint ? `\n(${hint})` : ""}`;

  return (
    <Tooltip content={content} position="top">
      {children}
    </Tooltip>
  );
}
