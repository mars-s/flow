import { useEffect } from "react";
import { motion } from "framer-motion";
import "./UndoToast.css";

// 10s window, matching PRD §6.1's own deletion-undo spec — the GPUI app
// reuses the same window for completion too (docs/HANDOFF.md), so this
// does the same rather than inventing a shorter one.
const UNDO_WINDOW_MS = 10_000;

export type UndoState = {
  message: string;
  onUndo: () => void;
};

type Props = {
  toast: UndoState | null;
  onDismiss: () => void;
};

export function UndoToast({ toast, onDismiss }: Props) {
  useEffect(() => {
    if (!toast) return;
    const timeout = setTimeout(onDismiss, UNDO_WINDOW_MS);
    return () => clearTimeout(timeout);
    // Re-arms whenever a *new* toast replaces the old one (message changes),
    // not on every render — same "a second action inside the first one's
    // window shouldn't let the first timer clear the second toast early"
    // reasoning the GPUI app's own show_undo_toast comment gives.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [toast?.message]);

  if (!toast) return null;

  return (
    <motion.div
      className="undo-toast"
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: 8 }}
      transition={{ type: "spring", stiffness: 480, damping: 36 }}
    >
      <span>{toast.message}</span>
      <button
        className="undo-toast-button"
        onClick={() => {
          toast.onUndo();
          onDismiss();
        }}
      >
        Undo
      </button>
    </motion.div>
  );
}
