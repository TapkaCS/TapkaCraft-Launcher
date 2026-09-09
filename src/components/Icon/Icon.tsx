/**
 * A small set of hand-drawn geometric line icons, kept as inline SVG (crisp
 * at any DPI/Windows display-scaling level, no icon-font/library
 * dependency). Not a copy of any Minecraft or third-party icon set.
 */

export type IconName =
  | "play"
  | "grid"
  | "compass"
  | "box"
  | "sliders"
  | "star"
  | "starOutline"
  | "chevronDown"
  | "folder"
  | "close"
  | "pencil"
  | "download"
  | "microsoft";

interface IconProps {
  name: IconName;
  size?: number;
  className?: string;
}

export function Icon({ name, size = 18, className }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      className={className}
      aria-hidden="true"
      focusable="false"
    >
      {renderPaths(name)}
    </svg>
  );
}

function renderPaths(name: IconName) {
  switch (name) {
    case "play":
      return <polygon points="6,3 21,12 6,21" fill="currentColor" />;
    case "grid":
      return (
        <>
          <rect x="3" y="3" width="8" height="8" fill="currentColor" />
          <rect x="13" y="3" width="8" height="8" fill="currentColor" />
          <rect x="3" y="13" width="8" height="8" fill="currentColor" />
          <rect x="13" y="13" width="8" height="8" fill="currentColor" />
        </>
      );
    case "compass":
      return (
        <>
          <circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" strokeWidth="1.8" />
          <polygon points="12,6.5 14.2,12 12,17.5 9.8,12" fill="currentColor" />
        </>
      );
    case "box":
      return (
        <>
          <path
            d="M12 3 L21 7.5 L21 16.5 L12 21 L3 16.5 L3 7.5 Z"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.6"
            strokeLinejoin="round"
          />
          <path
            d="M12 3 L12 21 M3 7.5 L12 12 L21 7.5"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.6"
            strokeLinejoin="round"
          />
        </>
      );
    case "sliders":
      return (
        <>
          <line x1="3" y1="6" x2="21" y2="6" stroke="currentColor" strokeWidth="2" />
          <circle cx="15" cy="6" r="2.4" fill="currentColor" />
          <line x1="3" y1="12" x2="21" y2="12" stroke="currentColor" strokeWidth="2" />
          <circle cx="9" cy="12" r="2.4" fill="currentColor" />
          <line x1="3" y1="18" x2="21" y2="18" stroke="currentColor" strokeWidth="2" />
          <circle cx="17" cy="18" r="2.4" fill="currentColor" />
        </>
      );
    case "star":
      return (
        <polygon
          points="12,2 14.7,8.6 22,9.2 16.5,13.8 18.2,21 12,17.1 5.8,21 7.5,13.8 2,9.2 9.3,8.6"
          fill="currentColor"
        />
      );
    case "starOutline":
      return (
        <polygon
          points="12,2 14.7,8.6 22,9.2 16.5,13.8 18.2,21 12,17.1 5.8,21 7.5,13.8 2,9.2 9.3,8.6"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.4"
          strokeLinejoin="round"
        />
      );
    case "chevronDown":
      return (
        <path
          d="M6 9 L12 15 L18 9"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "folder":
      return (
        <path
          d="M3 7a1 1 0 0 1 1-1h5l2 2h9a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V7z"
          fill="currentColor"
        />
      );
    case "close":
      return (
        <path
          d="M6 6 L18 18 M18 6 L6 18"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
        />
      );
    case "pencil":
      return (
        <path
          d="M4 20 L4 16.5 L15.5 5 A2.1 2.1 0 0 1 18.5 5 L19 5.5 A2.1 2.1 0 0 1 19 8.5 L7.5 20 Z M13.5 7 L17 10.5"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.6"
          strokeLinejoin="round"
          strokeLinecap="round"
        />
      );
    case "download":
      return (
        <path
          d="M12 3 L12 14 M7 9.5 L12 14.5 L17 9.5 M4 18 L4 20 A1 1 0 0 0 5 21 L19 21 A1 1 0 0 0 20 20 L20 18"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.8"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "microsoft":
      return (
        <>
          <rect x="2" y="2" width="9.2" height="9.2" fill="#F25022" />
          <rect x="12.8" y="2" width="9.2" height="9.2" fill="#7FBA00" />
          <rect x="2" y="12.8" width="9.2" height="9.2" fill="#00A4EF" />
          <rect x="12.8" y="12.8" width="9.2" height="9.2" fill="#FFB900" />
        </>
      );
  }
}
