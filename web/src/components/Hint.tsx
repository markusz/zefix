// An info button with an explanation. Opens on hover, keyboard focus or tap, so it works on
// phones too, and keeps its bubble inside the window.

import { useId, useLayoutEffect, useRef, useState } from "react";

/** Distance of the bubble from the button and from the window edges. */
const GAP = 8;
const MAX_WIDTH = 300;

export function Hint({ children, label = "Erklärung" }: { children: React.ReactNode; label?: string }) {
  const id = useId();
  const button = useRef<HTMLButtonElement>(null);
  const bubble = useRef<HTMLSpanElement>(null);
  const [open, setOpen] = useState(false);
  const [pinned, setPinned] = useState(false);
  const [position, setPosition] = useState<{ left: number; top: number }>({ left: 0, top: 0 });

  useLayoutEffect(() => {
    if (!open || !button.current || !bubble.current) return;
    const anchor = button.current.getBoundingClientRect();
    const { width, height } = bubble.current.getBoundingClientRect();
    const left = Math.min(
      Math.max(GAP, anchor.left + anchor.width / 2 - width / 2),
      window.innerWidth - width - GAP,
    );
    const below = anchor.bottom + GAP;
    const top = below + height > window.innerHeight - GAP ? anchor.top - GAP - height : below;
    setPosition({ left, top });
  }, [open]);

  // A tap pins the bubble open; the next tap anywhere closes it.
  useLayoutEffect(() => {
    if (!pinned) return;
    const close = (event: PointerEvent) => {
      if (event.target instanceof Node && button.current?.contains(event.target)) return;
      setPinned(false);
      setOpen(false);
    };
    document.addEventListener("pointerdown", close);
    return () => document.removeEventListener("pointerdown", close);
  }, [pinned]);

  return (
    <span className="hint">
      <button
        ref={button}
        type="button"
        className="hint-button"
        aria-label={label}
        aria-describedby={open ? id : undefined}
        aria-expanded={open}
        onMouseEnter={() => setOpen(true)}
        onMouseLeave={() => pinned || setOpen(false)}
        onFocus={() => setOpen(true)}
        onBlur={() => pinned || setOpen(false)}
        onClick={(e) => {
          // Inside a <summary> the click would otherwise also toggle the section.
          e.preventDefault();
          setPinned(!pinned);
          setOpen(!pinned);
        }}
        onKeyDown={(e) => e.key === "Escape" && (setOpen(false), setPinned(false))}
      >
        i
      </button>
      {open && (
        <span
          ref={bubble}
          id={id}
          role="tooltip"
          className="hint-bubble"
          style={{ left: position.left, top: position.top, maxWidth: MAX_WIDTH }}
        >
          {children}
        </span>
      )}
    </span>
  );
}
