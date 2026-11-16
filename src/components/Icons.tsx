// Pictogrammes au trait, dans l'esprit des SF Symbols (grille 24, trait 1.8, extrémités rondes).

import type { ReactNode, SVGProps } from "react";

function make(children: ReactNode, filled = false) {
  return function Icon(props: SVGProps<SVGSVGElement>) {
    return (
      <svg
        viewBox="0 0 24 24"
        fill={filled ? "currentColor" : "none"}
        stroke={filled ? "none" : "currentColor"}
        strokeWidth={1.8}
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
        {...props}
      >
        {children}
      </svg>
    );
  };
}

export const IconGrid = make(
  <>
    <rect x="3.5" y="3.5" width="7" height="7" rx="2" />
    <rect x="13.5" y="3.5" width="7" height="7" rx="2" />
    <rect x="3.5" y="13.5" width="7" height="7" rx="2" />
    <rect x="13.5" y="13.5" width="7" height="7" rx="2" />
  </>,
);
export const IconHeart = make(
  <path d="M12 20.3s-7.5-4.4-7.5-10A4.4 4.4 0 0 1 12 7.6a4.4 4.4 0 0 1 7.5 2.7c0 5.6-7.5 10-7.5 10Z" />,
);
export const IconHeartFill = make(
  <path d="M12 20.8s-8.2-4.7-8.2-10.6A4.9 4.9 0 0 1 12 6.9a4.9 4.9 0 0 1 8.2 3.3c0 5.9-8.2 10.6-8.2 10.6Z" />,
  true,
);
export const IconClock = make(
  <>
    <circle cx="12" cy="12" r="8.5" />
    <path d="M12 7.5V12l3 2" />
  </>,
);
export const IconBag = make(
  <>
    <path d="M5 8h14l-1 11.2a2 2 0 0 1-2 1.8H8a2 2 0 0 1-2-1.8L5 8Z" />
    <path d="M9 8V6.5a3 3 0 0 1 6 0V8" />
  </>,
);
export const IconSearch = make(
  <>
    <circle cx="10.8" cy="10.8" r="6.3" />
    <path d="m15.6 15.6 4.4 4.4" />
  </>,
);
export const IconGear = make(
  <>
    <circle cx="12" cy="12" r="3" />
    <path d="M12 2.8v2.4M12 18.8v2.4M4 7.4l2.1 1.2M17.9 15.4 20 16.6M4 16.6l2.1-1.2M17.9 8.6 20 7.4" />
    <circle cx="12" cy="12" r="6.8" />
  </>,
);
export const IconPlay = make(<path d="M8 5.2v13.6a1 1 0 0 0 1.5.86l11.3-6.8a1 1 0 0 0 0-1.72L9.5 4.34A1 1 0 0 0 8 5.2Z" />, true);
export const IconClose = make(<path d="m6 6 12 12M18 6 6 18" />);
export const IconMinus = make(<path d="M5.5 12h13" />);
export const IconSquare = make(<rect x="5.5" y="5.5" width="13" height="13" rx="2.6" />);
export const IconRefresh = make(
  <>
    <path d="M19.5 12a7.5 7.5 0 1 1-2.6-5.7" />
    <path d="M19.8 4.2v4.3h-4.3" />
  </>,
);
export const IconFolder = make(
  <path d="M3.5 7.5a2 2 0 0 1 2-2h3.6l2 2.2h7.4a2 2 0 0 1 2 2v7.8a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2v-10Z" />,
);
export const IconEyeOff = make(
  <>
    <path d="M4 4.5 20 19.5" />
    <path d="M9.9 6.2A9.4 9.4 0 0 1 12 6c5.2 0 8.5 6 8.5 6a14.7 14.7 0 0 1-2.4 3.1M6.4 8.1A14.6 14.6 0 0 0 3.5 12s3.3 6 8.5 6c1 0 2-.2 2.8-.5" />
    <path d="M10 10.2a2.8 2.8 0 0 0 3.9 3.9" />
  </>,
);
export const IconEye = make(
  <>
    <path d="M3.5 12S6.8 6 12 6s8.5 6 8.5 6-3.3 6-8.5 6-8.5-6-8.5-6Z" />
    <circle cx="12" cy="12" r="2.8" />
  </>,
);
export const IconMore = make(
  <>
    <circle cx="6" cy="12" r="1.4" fill="currentColor" stroke="none" />
    <circle cx="12" cy="12" r="1.4" fill="currentColor" stroke="none" />
    <circle cx="18" cy="12" r="1.4" fill="currentColor" stroke="none" />
  </>,
);
export const IconArrowOut = make(<path d="M8 16 16.5 7.5M9.5 7.5h7v7" />);
export const IconChevron = make(<path d="m9.5 6 6 6-6 6" />);
export const IconPlus = make(<path d="M12 5.5v13M5.5 12h13" />);
export const IconTrash = make(
  <>
    <path d="M5 7h14M10 7V5.2A1.2 1.2 0 0 1 11.2 4h1.6A1.2 1.2 0 0 1 14 5.2V7" />
    <path d="m6.5 7 .8 11.2a2 2 0 0 0 2 1.8h5.4a2 2 0 0 0 2-1.8L17.500 7" />
  </>,
);
export const IconController = make(
  <>
    <path d="M7.5 8h9a4.5 4.5 0 0 1 4.4 3.6l.8 4a2.4 2.4 0 0 1-4.3 1.9L16 16H8l-1.4 1.5a2.4 2.4 0 0 1-4.3-1.9l.8-4A4.5 4.5 0 0 1 7.500 8Z" />
    <path d="M7.300 11v2.600M6 12.300h2.600" />
    <circle cx="15.400" cy="11.600" r=".9" fill="currentColor" stroke="none" />
    <circle cx="17.300" cy="13.500" r=".9" fill="currentColor" stroke="none" />
  </>,
);
export const IconApp = make(
  <>
    <rect x="3.500" y="5" width="17" height="14" rx="3" />
    <path d="M3.500 9.500h17" />
  </>,
);

/** Logo de l'application (même dessin que l'icône du programme). */
export function AppLogo() {
  return (
    <svg viewBox="0 0 1024 1024" aria-hidden="true">
      <defs>
        <linearGradient id="logo-bg" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#3AA0FF" />
          <stop offset="0.52" stopColor="#5E5CE6" />
          <stop offset="1" stopColor="#C05AF2" />
        </linearGradient>
      </defs>
      <rect x="40" y="40" width="944" height="944" rx="230" fill="url(#logo-bg)" />
      <path d="M436 340 L436 684 L722 512 Z" fill="#fff" stroke="#fff" strokeWidth="84" strokeLinejoin="round" />
    </svg>
  );
}

export const IconLock = make(
  <>
    <rect x="5" y="10.5" width="14" height="10" rx="2.6" />
    <path d="M8.5 10.5V7.8a3.5 3.5 0 0 1 7 0v2.7" />
  </>,
);

export const IconCheck = make(<path d="m5.5 12.5 4.2 4.2 8.8-9.4" />);
export const IconCollection = make(
  <>
    <rect x="4" y="8.5" width="16" height="11.5" rx="2.6" />
    <path d="M6.5 5.5h11M9 3h6" />
  </>,
);
export const IconDisk = make(
  <>
    <rect x="3.5" y="6" width="17" height="12" rx="3" />
    <path d="M7 14.5h4" />
    <circle cx="16.5" cy="14.5" r=".9" fill="currentColor" stroke="none" />
  </>,
);
export const IconTv = make(
  <>
    <rect x="3" y="4.5" width="18" height="12.5" rx="2.6" />
    <path d="M8.5 20h7" />
  </>,
);
export const IconImage = make(
  <>
    <rect x="3.5" y="4.5" width="17" height="15" rx="2.8" />
    <circle cx="9" cy="10" r="1.6" />
    <path d="m4.5 18 5-5 3.5 3.5 2.5-2.5 4 4" />
  </>,
);
export const IconPencil = make(<path d="M15.5 5.5 18.5 8.5M5 19l1-4L16.3 4.7a1.5 1.5 0 0 1 2.1 0l.9.9a1.5 1.5 0 0 1 0 2.1L9 18l-4 1Z" />);
