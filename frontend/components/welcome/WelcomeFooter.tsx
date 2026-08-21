/** GitHub link -- Keyboard Shortcuts lives in the Start action list as a real action row now,
 *  so this footer is just the one external link. Pinned to the bottom-left of the viewport via
 *  WelcomeScreen.css, not flowing after Recent (see that file's doc). */
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
