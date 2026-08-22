import "./WelcomeHeader.css";

/**
 * Welcome screen header -- logo, app name, tagline. Pure presentation.
 */
export function WelcomeHeader() {
  return (
    <div className="welcome-header">
      <div className="welcome-logo" aria-hidden="true">
        <svg viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
          <rect x="8" y="12" width="24" height="56" rx="4" fill="url(#grad1)" />
          <rect
            x="28"
            y="20"
            width="24"
            height="48"
            rx="4"
            fill="url(#grad1)"
            opacity="0.8"
          />
          <rect
            x="48"
            y="28"
            width="24"
            height="40"
            rx="4"
            fill="url(#grad1)"
            opacity="0.6"
          />
          <defs>
            <linearGradient
              id="grad1"
              x1="8"
              y1="12"
              x2="72"
              y2="68"
              gradientUnits="userSpaceOnUse"
            >
              <stop stopColor="#007acc" />
              <stop offset="1" stopColor="#4a9eff" />
            </linearGradient>
          </defs>
        </svg>
      </div>
      <div className="welcome-header-text">
        <h1 className="welcome-title">Bitvue</h1>
        <p className="welcome-subtitle">Video Bitstream Analyzer</p>
      </div>
    </div>
  );
}
