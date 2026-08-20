import { motion } from "framer-motion";
import { Calendar, Inbox, Archive, Trash2 } from "lucide-react";
import "./BulkActionBar.css";

export type BulkTarget = "today" | "anytime" | "someday";

type Props = {
  count: number;
  onProcess: (target: BulkTarget) => void;
  onDelete: () => void;
};

// Mirrors the GPUI app's own bulk_action_bar: Today/Anytime/Someday reuse
// the sidebar's icon for the same destination concept, Delete is the same
// shared button reachable after selecting several rows.
export function BulkActionBar({ count, onProcess, onDelete }: Props) {
  return (
    <motion.div
      className="bulk-action-bar"
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: 12 }}
      transition={{ type: "spring", stiffness: 480, damping: 36 }}
      onClick={(event) => event.stopPropagation()}
    >
      <span className="bulk-action-count">{count} selected</span>
      <div className="bulk-action-buttons">
        <button type="button" onClick={() => onProcess("today")}>
          <Calendar size={13} />
          Today
        </button>
        <button type="button" onClick={() => onProcess("anytime")}>
          <Inbox size={13} />
          Anytime
        </button>
        <button type="button" onClick={() => onProcess("someday")}>
          <Archive size={13} />
          Someday
        </button>
        <button type="button" className="bulk-action-delete" onClick={onDelete}>
          <Trash2 size={13} />
          Delete
        </button>
      </div>
    </motion.div>
  );
}
