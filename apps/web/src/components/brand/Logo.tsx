import type React from 'react';

type LogoProps = {
  size?: number;
  variant?: 'mark' | 'full';
  className?: string;
};

export function Logo({ size = 64, variant = 'mark', className }: LogoProps): React.JSX.Element {
  if (variant === 'full') {
    return (
      <svg
        viewBox="0 0 320 96"
        height={size}
        width={size * (320 / 96)}
        role="img"
        aria-label="Mneme"
        className={className}
      >
        <title>Mneme</title>
        <circle
          cx="48"
          cy="56"
          r="36"
          fill="none"
          stroke="var(--color-mneme-cyan)"
          strokeWidth="1.5"
          opacity=".3"
        />
        <circle
          cx="48"
          cy="56"
          r="28"
          fill="none"
          stroke="var(--color-mneme-cyan)"
          strokeWidth="1.5"
          opacity=".5"
        />
        <circle
          cx="48"
          cy="56"
          r="20"
          fill="none"
          stroke="var(--color-mneme-cyan)"
          strokeWidth="2"
        />
        <circle cx="48" cy="56" r="12" fill="var(--color-mneme-cyan)" opacity=".15" />
        <path
          d="M 48 14 C 48 14, 38 30, 38 38 C 38 44, 42 48, 48 48 C 54 48, 58 44, 58 38 C 58 30, 48 14, 48 14 Z"
          fill="var(--color-mneme-gold)"
        />
        <text
          x="108"
          y="62"
          fontFamily="Inter, system-ui, sans-serif"
          fontWeight="800"
          fontSize="32"
          letterSpacing="6"
          fill="var(--color-mneme-text)"
        >
          MNEME
        </text>
      </svg>
    );
  }

  return (
    <svg
      viewBox="0 0 96 96"
      height={size}
      width={size}
      role="img"
      aria-label="Mneme"
      className={className}
    >
      <title>Mneme</title>
      <circle
        cx="48"
        cy="56"
        r="36"
        fill="none"
        stroke="var(--color-mneme-cyan)"
        strokeWidth="1.5"
        opacity=".3"
      />
      <circle
        cx="48"
        cy="56"
        r="28"
        fill="none"
        stroke="var(--color-mneme-cyan)"
        strokeWidth="1.5"
        opacity=".5"
      />
      <circle cx="48" cy="56" r="20" fill="none" stroke="var(--color-mneme-cyan)" strokeWidth="2" />
      <circle cx="48" cy="56" r="12" fill="var(--color-mneme-cyan)" opacity=".15" />
      <path
        d="M 48 14 C 48 14, 38 30, 38 38 C 38 44, 42 48, 48 48 C 54 48, 58 44, 58 38 C 58 30, 48 14, 48 14 Z"
        fill="var(--color-mneme-gold)"
      />
      <circle cx="48" cy="40" r="2.5" fill="var(--color-mneme-bg)" opacity=".4" />
    </svg>
  );
}
