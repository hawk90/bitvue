import "./WelcomeFooter.css";

/** GitHub link -- Keyboard Shortcuts lives in the Start action list as a real action row now,
 *  so this footer is just the one external link. Flows after Samples/Practice, not pinned to
 *  the viewport (see WelcomeFooter.css's module doc). */
export function WelcomeFooter() {
  return (
    <div className="welcome-footer">
      <div className="footer-links">
        <a
          href="https://github.com/hawk90/bitvue"
          target="_blank"
          rel="noopener noreferrer"
        >
          <span className="codicon codicon-mark-github" aria-hidden="true" />
          GitHub
        </a>
      </div>
    </div>
  );
}
