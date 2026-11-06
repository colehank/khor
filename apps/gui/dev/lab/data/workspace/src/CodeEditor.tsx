import {EditorState} from '@codemirror/state';
import {keymap} from '@codemirror/view';
import {defaultKeymap,history,historyKeymap} from '@codemirror/commands';

const extensions = [history(),keymap.of([...historyKeymap,...defaultKeymap])];

export function createEditor(doc: string) {
  return EditorState.create({doc,extensions});
}
