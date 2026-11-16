interface IconProps {
  size?: number;
  className?: string;
}

// 导入本地 SVG 图标
import ClaudeSvg from "@/icons/extracted/claude.svg?url";
import OpenAISvg from "@/icons/extracted/openai.svg?url";
import GeminiSvg from "@/icons/extracted/gemini.svg?url";
import OpenClawSvg from "@/icons/extracted/claw.svg?url";

export function ClaudeIcon({ size = 16, className = "" }: IconProps) {
  return (
    <img
      src={ClaudeSvg}
      width={size}
      height={size}
      className={className}
      alt="Claude"
      loading="lazy"
    />
  );
}

export function CodexIcon({ size = 16, className = "" }: IconProps) {
  return (
    <img
      src={OpenAISvg}
      width={size}
      height={size}
      className={`dark:brightness-0 dark:invert ${className}`}
      alt="Codex"
      loading="lazy"
    />
  );
}

export function GeminiIcon({ size = 16, className = "" }: IconProps) {
  return (
    <img
      src={GeminiSvg}
      width={size}
      height={size}
      className={className}
      alt="Gemini"
      loading="lazy"
    />
  );
}

export function OpenClawIcon({ size = 16, className = "" }: IconProps) {
  return (
    <img
      src={OpenClawSvg}
      width={size}
      height={size}
      className={className}
      alt="OpenClaw"
      loading="lazy"
    />
  );
}

// Claude Desktop-style Skills scroll, based on the reference image.
/** ZCode（Z.AI）品牌图标：深底圆角方块 + 渐变 Z。 */
export function ZCodeIcon({ size = 16, className = "" }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 64 64"
      className={className}
      style={{ flex: "none", lineHeight: 1 }}
      aria-hidden="true"
    >
      <defs>
        <linearGradient id="zcode-g" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#5ad8e6" />
          <stop offset=".5" stopColor="#7c6bff" />
          <stop offset="1" stopColor="#b44dff" />
        </linearGradient>
      </defs>
      <rect width="64" height="64" rx="14" fill="#0b0e14" />
      <text
        x="32"
        y="44"
        textAnchor="middle"
        fontFamily="Arial, 'Segoe UI', sans-serif"
        fontSize="38"
        fontWeight="800"
        fill="url(#zcode-g)"
      >
        Z
      </text>
    </svg>
  );
}

/** DeepSeek Harness（DSH）品牌图标：科技蓝渐变底 + 极客造型。 */
export function DshIcon({ size = 16, className = "" }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 64 64"
      className={className}
      style={{ flex: "none", lineHeight: 1 }}
      aria-hidden="true"
    >
      <defs>
        <linearGradient id="dsh-bg" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#0d1b2a" />
          <stop offset="1" stopColor="#1b263b" />
        </linearGradient>
        <linearGradient id="dsh-text" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#4cc9f0" />
          <stop offset="1" stopColor="#4361ee" />
        </linearGradient>
      </defs>
      <rect width="64" height="64" rx="14" fill="url(#dsh-bg)" />
      <text
        x="32"
        y="42"
        textAnchor="middle"
        fontFamily="system-ui, -apple-system, sans-serif"
        fontSize="24"
        fontWeight="800"
        letterSpacing="-1"
        fill="url(#dsh-text)"
      >
        DSH
      </text>
    </svg>
  );
}

export function SkillsIcon({ size = 16, className = "" }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.75}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
      focusable="false"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path d="M4 3h14a2 2 0 0 1 2 2v12" />
      <path d="M2 7V5a2 2 0 0 1 4 0v14a2 2 0 0 0 4 0v-2h12v2a2 2 0 0 1-2 2H8" />
      <path d="m9 10.5 1.5-2-.5 2 2-1.5-.5 1.5 2-1.5-.5 1.5 2-2" />
    </svg>
  );
}

// MCP icon uses inline SVG to support currentColor for hover effects
export function McpIcon({ size = 16, className = "" }: IconProps) {
  return (
    <svg
      fill="currentColor"
      fillRule="evenodd"
      height={size}
      width={size}
      className={className}
      viewBox="0 0 24 24"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path d="M15.688 2.343a2.588 2.588 0 00-3.61 0l-9.626 9.44a.863.863 0 01-1.203 0 .823.823 0 010-1.18l9.626-9.44a4.313 4.313 0 016.016 0 4.116 4.116 0 011.204 3.54 4.3 4.3 0 013.609 1.18l.05.05a4.115 4.115 0 010 5.9l-8.706 8.537a.274.274 0 000 .393l1.788 1.754a.823.823 0 010 1.18.863.863 0 01-1.203 0l-1.788-1.753a1.92 1.92 0 010-2.754l8.706-8.538a2.47 2.47 0 000-3.54l-.05-.049a2.588 2.588 0 00-3.607-.003l-7.172 7.034-.002.002-.098.097a.863.863 0 01-1.204 0 .823.823 0 010-1.18l7.273-7.133a2.47 2.47 0 00-.003-3.537z" />
      <path d="M14.485 4.703a.823.823 0 000-1.18.863.863 0 00-1.204 0l-7.119 6.982a4.115 4.115 0 000 5.9 4.314 4.314 0 006.016 0l7.12-6.982a.823.823 0 000-1.18.863.863 0 00-1.204 0l-7.119 6.982a2.588 2.588 0 01-3.61 0 2.47 2.47 0 010-3.54l7.12-6.982z" />
    </svg>
  );
}
