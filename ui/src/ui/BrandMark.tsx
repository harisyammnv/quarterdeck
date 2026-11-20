/** The Raised deck mark (design system logos), single ink: it takes the current text colour. */
export function BrandMark({ size = 24 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 64 64" aria-hidden="true" focusable="false">
      <g fill="currentColor" transform="translate(0 6)">
        <path d="M3 31H37V21H61V35Q61 37 59.5 38.5L54 44Q52.5 46 50 46H13Q11 46 10 44Z" />
        <path d="M40 16V6L50 11Z" />
        <rect x="39" y="6" width="2.5" height="15" />
      </g>
    </svg>
  );
}
