import { Check, Palette } from "lucide-react";
import { THEMES, type ThemeId } from "../lib/theme";
import "./ThemeSwitcher.css";

type Props = {
  theme: ThemeId;
  onChange: (theme: ThemeId) => void;
};

export function ThemeSwitcher({ theme, onChange }: Props) {
  return (
    <div className="settings-section">
      <div className="settings-row">
        <div className="settings-row-icon">
          <Palette size={16} />
        </div>
        <div className="settings-row-body">
          <div className="settings-row-title">Appearance</div>
          <div className="settings-row-note">Applies immediately, and remembers your choice.</div>
        </div>
      </div>
      <div className="theme-switcher-options">
        {THEMES.map((option) => (
          <button
            type="button"
            key={option.id}
            className={option.id === theme ? "theme-option active" : "theme-option"}
            onClick={() => onChange(option.id)}
          >
            <span className="theme-option-swatch" style={{ background: option.accent }}>
              {option.id === theme && <Check size={13} color="#fff" strokeWidth={3} />}
            </span>
            <span className="theme-option-body">
              <span className="theme-option-name">{option.name}</span>
              <span className="theme-option-description">{option.description}</span>
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}
