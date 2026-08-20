import type { AiFeatureState } from "../lib/aiConfig";
import "./AIFeatureToggle.css";

const STATES: { id: AiFeatureState; label: string }[] = [
  { id: "auto", label: "Auto" },
  { id: "manual", label: "Manual" },
  { id: "off", label: "Off" },
];

type Props = {
  value: AiFeatureState;
  onChange: (state: AiFeatureState) => void;
};

// The per-feature three-state control every AI block gets (originally
// built vertical per an initial request, corrected to horizontal — the
// user's actual intent all along). Auto, Manual, Off reading left-to-
// right as "most to least autonomous."
export function AIFeatureToggle({ value, onChange }: Props) {
  const activeIndex = STATES.findIndex((state) => state.id === value);

  return (
    <div className="ai-feature-toggle" role="radiogroup" aria-label="AI feature mode">
      <div className="ai-feature-toggle-thumb" style={{ transform: `translateX(${activeIndex * 100}%)` }} />
      {STATES.map((state) => (
        <button
          key={state.id}
          type="button"
          role="radio"
          aria-checked={value === state.id}
          className={value === state.id ? "ai-feature-toggle-option active" : "ai-feature-toggle-option"}
          onClick={() => onChange(state.id)}
        >
          {state.label}
        </button>
      ))}
    </div>
  );
}
