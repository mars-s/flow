import { useEffect, useRef } from "react";
import { motion } from "framer-motion";
import "./CaptureField.css";

type Props = {
  open: boolean;
  onSubmit: (title: string) => void;
  onClose: () => void;
};

export function CaptureField({ open, onSubmit, onClose }: Props) {
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (open) inputRef.current?.focus();
  }, [open]);

  if (!open) return null;

  return (
    <motion.form
      className="capture-field"
      initial={{ opacity: 0, y: -6 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -6 }}
      transition={{ type: "spring", stiffness: 500, damping: 36 }}
      onSubmit={(event) => {
        event.preventDefault();
        const value = inputRef.current?.value.trim();
        if (value) {
          onSubmit(value);
          if (inputRef.current) inputRef.current.value = "";
        }
      }}
    >
      <input
        ref={inputRef}
        className="capture-input"
        placeholder="Take out laundry 8am tomorrow…"
        onKeyDown={(event) => {
          if (event.key === "Escape") onClose();
        }}
        onBlur={onClose}
      />
    </motion.form>
  );
}
