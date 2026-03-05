import React from "react";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

type Props = { children: React.ReactNode; onReset: () => void };
type State = { hasError: boolean; error: Error | null };

export class AgentErrorBoundary extends React.Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error("[AgentErrorBoundary]", error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return (
        <Card className="flex flex-col flex-1 min-h-[200px] p-6 gap-4 border-destructive/50">
          <p className="text-sm font-medium">Something went wrong in Agent mode.</p>
          {this.state.error && (
            <pre className="text-xs text-muted-foreground overflow-auto max-h-24 p-2 bg-muted rounded">
              {this.state.error.message}
            </pre>
          )}
          <Button variant="outline" size="sm" onClick={() => this.setState({ hasError: false, error: null })}>
            Try again
          </Button>
          <Button variant="ghost" size="sm" onClick={this.props.onReset}>
            Switch Mode
          </Button>
        </Card>
      );
    }
    return this.props.children;
  }
}
