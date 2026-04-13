import { Component, type ReactNode } from "react";
import css from "./ErrorBoundary.module.css";

interface Props {
  section: string;
  children: ReactNode;
}

interface State {
  error: Error | null;
}

/**
 * Catches render errors in a subtree and shows a recoverable fallback UI
 * instead of crashing the entire application.
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error(`[${this.props.section}] render error:`, error, info.componentStack);
  }

  render() {
    if (this.state.error) {
      return (
        <div className={css.fallback}>
          <div className={css.icon}>!</div>
          <h3 className={css.title}>{this.props.section} error</h3>
          <p className={css.message}>{this.state.error.message}</p>
          <button
            className={css.retryButton}
            onClick={() => this.setState({ error: null })}
          >
            Retry
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
