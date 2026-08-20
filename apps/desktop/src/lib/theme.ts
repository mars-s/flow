export type ThemeId = "default" | "river-cut";

// Accent swatches shown in the theme switcher — same hex values theme.css
// itself defines per theme, just duplicated here since CSS custom
// properties aren't readable from JS without a live DOM node to query,
// and a swatch needs to render before that theme is ever applied.
export const THEMES: { id: ThemeId; name: string; accent: string; description: string }[] = [
  { id: "default", name: "Default", accent: "#69a9ff", description: "Flow's original cool blue-gray." },
  {
    id: "river-cut",
    name: "River Cut",
    accent: "#8fa688",
    description: "Built from the River Cut icon concept — sampled straight off the generated artwork.",
  },
];
