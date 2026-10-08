// **A part that fails to draw takes only itself down** (afix-a, 2026-10-08;
// the audit's R1: a node that answered `null` to `doctor` or `find` threw
// in a render, and with no boundary anywhere the whole window went to its
// bare ground — the rail, the list and every way out gone with the one
// page that broke).
//
// A seam is drawn where a person can still do something from the rest of
// the window: the app itself (`main.tsx`, the last one), each landing's
// board and its list, a 「…」 page, a session's side column, every dialog
// (`ui/dialog`) and ⌘K's hits (`SideHead`). What it shows is the error
// state of `RegionState` (guidelines/20-states 出错): one sentence where
// the empty state sits, and 重试. **Never the error's own text** — that
// is an engine's sentence about a line of code, and it goes to the
// console, which React 19 already writes for a caught error.
//
// 重试 draws the part again from scratch (its children remount, so what
// they fetch is asked again); moving somewhere else (`reset`) clears it
// too, so a landing that broke once is not broken for the rest of the
// session.
import { Component, Fragment, type ReactNode } from "react";

import { EmptyState } from "@/components/RegionState";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";

export class Fault extends Component<
  {
    /** The seam's name, on the drawn sentence (`data-fault`). */
    name: string;
    /** Where the part is: a new value clears a fault, the way leaving a
        page and coming back would. Compared by `Object.is`. */
    reset?: unknown;
    /** The sentence's box, where the seat is not a board (⌘K's hits). */
    className?: string;
    /** Drawn with the sentence: a phone's screen keeps its 「‹」 here,
        since the page that would have drawn it is the part that failed. */
    head?: ReactNode;
    children: ReactNode;
  },
  { failed: boolean; round: number }
> {
  state = { failed: false, round: 0 };

  static getDerivedStateFromError() {
    return { failed: true };
  }

  componentDidUpdate(was: { reset?: unknown }) {
    if (this.state.failed && !Object.is(was.reset, this.props.reset)) {
      this.setState((s) => ({ failed: false, round: s.round + 1 }));
    }
  }

  render() {
    if (this.state.failed) {
      return (
        <>
          {this.props.head}
          <EmptyState
            state="error"
            data-fault={this.props.name}
            className={this.props.className}
            action={
              <Button data-fault-retry onClick={() => this.setState((s) => ({ failed: false, round: s.round + 1 }))}>
                {gui.region_retry}
              </Button>
            }
          >
            {gui.fault_said}
          </EmptyState>
        </>
      );
    }
    return <Fragment key={this.state.round}>{this.props.children}</Fragment>;
  }
}
