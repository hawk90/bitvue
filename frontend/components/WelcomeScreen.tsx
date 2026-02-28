/**
 * Simple Welcome Screen
 */

import { memo, useEffect } from "react";
import "./WelcomeScreen.css";

interface WelcomeScreenProps {
  onOpenFile: () => void;
  loading: boolean;
  error: string | null;
}

export const WelcomeScreen = memo(function WelcomeScreen({
  onOpenFile,
  loading,
  error,
}: WelcomeScreenProps) {
  console.log("[WelcomeScreen] Rendering", {
    loading,
    error,
    onOpenFile: typeof onOpenFile,
  });

  useEffect(() => {
    console.log("[WelcomeScreen] Mounted");
    return () => console.log("[WelcomeScreen] Unmounted");
  }, []);

  return (
    <div className="welcome-screen" data-testid="welcome-screen">
      <div className="welcome-content">
        {/* Header */}
        <div className="welcome-header">
          <div className="welcome-logo">
            <svg
              viewBox="0 0 80 80"
              fill="none"
              xmlns="http://www.w3.org/2000/svg"
            >
              <rect
                x="8"
                y="12"
                width="24"
                height="56"
                rx="4"
                fill="url(#grad1)"
              />
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
          <h1 className="welcome-title">Bitvue</h1>
          <p className="welcome-subtitle">Video Bitstream Analyzer</p>
          <span className="welcome-badge">Feature Complete</span>
        </div>

        {/* Actions */}
        <div className="welcome-actions">
          <button
            className="welcome-open-btn"
            onClick={onOpenFile}
            disabled={loading}
          >
            {loading ? (
              <>
                <div className="btn-icon spinner">
                  <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                  >
                    <circle cx="12" cy="12" r="10" strokeOpacity="0.25" />
                    <path d="M12 2a10 10 0 0 1 10 10" strokeLinecap="round" />
                  </svg>
                </div>
                Opening...
              </>
            ) : (
              <>
                <div className="btn-icon">
                  <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                  >
                    <path
                      d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M17 8l-5-5-5 5M12 3v12"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    />
                  </svg>
                </div>
                Open Bitstream File
              </>
            )}
          </button>

          {error && (
            <div className="welcome-error">
              <span className="codicon codicon-error" />
              {error}
            </div>
          )}

          <div className="welcome-shortcuts">
            <kbd>⌘</kbd>
            <span>+</span>
            <kbd>O</kbd>
            <span>to open a file</span>
          </div>
        </div>

        {/* Features */}
        <div className="welcome-features">
          <div className="feature-card">
            <div className="feature-icon">
              <span className="codicon codicon-film-strip" />
            </div>
            <div className="feature-text">
              <div className="feature-title">Multi-Codec Support</div>
              <div className="feature-desc">VVC, HEVC, AV1, VP9, AVC</div>
            </div>
          </div>

          <div className="feature-card">
            <div className="feature-icon">
              <span className="codicon codicon-dashboard" />
            </div>
            <div className="feature-text">
              <div className="feature-title">Visualization Modes</div>
              <div className="feature-desc">
                Coding flow, prediction, transform
              </div>
            </div>
          </div>

          <div className="feature-card">
            <div className="feature-icon">
              <span className="codicon codicon-graph-line" />
            </div>
            <div className="feature-text">
              <div className="feature-title">Frame Analysis</div>
              <div className="feature-desc">
                Size, QP, MV field visualization
              </div>
            </div>
          </div>

          <div className="feature-card">
            <div className="feature-icon">
              <span className="codicon codicon-symbol-reference" />
            </div>
            <div className="feature-text">
              <div className="feature-title">Reference Tracking</div>
              <div className="feature-desc">
                Frame dependencies & GOP structure
              </div>
            </div>
          </div>
        </div>

        {/* Supported Formats */}
        <div className="welcome-formats">
          <div className="formats-title">Supported Codecs</div>
          <div className="formats-grid">
            <div
              className="format-item"
              style={{ "--format-color": "#e03131" } as React.CSSProperties}
            >
              <div className="format-dot" />
              <span className="format-codec">VVC</span>
              <span className="format-extensions">.h266 .vvc</span>
            </div>
            <div
              className="format-item"
              style={{ "--format-color": "#007acc" } as React.CSSProperties}
            >
              <div className="format-dot" />
              <span className="format-codec">HEVC</span>
              <span className="format-extensions">.h265 .hevc</span>
            </div>
            <div
              className="format-item"
              style={{ "--format-color": "#2da44e" } as React.CSSProperties}
            >
              <div className="format-dot" />
              <span className="format-codec">AV1</span>
              <span className="format-extensions">.av1 .ivf</span>
            </div>
            <div
              className="format-item"
              style={{ "--format-color": "#d4a717" } as React.CSSProperties}
            >
              <div className="format-dot" />
              <span className="format-codec">VP9</span>
              <span className="format-extensions">.vp9 .webm</span>
            </div>
            <div
              className="format-item"
              style={{ "--format-color": "#f14c4c" } as React.CSSProperties}
            >
              <div className="format-dot" />
              <span className="format-codec">AVC</span>
              <span className="format-extensions">.h264 .avc</span>
            </div>
            <div
              className="format-item"
              style={{ "--format-color": "#75beff" } as React.CSSProperties}
            >
              <div className="format-dot" />
              <span className="format-codec">MPEG-2</span>
              <span className="format-extensions">.mp2 .mpg</span>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="welcome-footer">
          <div className="footer-links">
            <a
              href="https://github.com/hawkw/bitvue"
              target="_blank"
              rel="noopener noreferrer"
            >
              <span className="codicon codicon-mark-github" />
              GitHub
            </a>
            <span className="footer-divider">•</span>
            <a
              href="#"
              onClick={(e) => {
                e.preventDefault();
                window.dispatchEvent(
                  new CustomEvent("menu-keyboard-shortcuts"),
                );
              }}
            >
              <span className="codicon codicon-keyboard" />
              Shortcuts
            </a>
          </div>
        </div>
      </div>
    </div>
  );
});
