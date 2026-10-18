/** Searchable choices share a field and a bounded scrolling menu. Free text
 * stays literal; selecting a row is the only way to choose an entity. */
import { useEffect, useId, useLayoutEffect, useRef, useState, type ReactNode, type KeyboardEvent, type ChangeEvent, type FocusEvent, type MouseEvent } from 'react';
import * as PopoverPrimitive from '@radix-ui/react-popover';
import { gui } from '@/gen/catalog';
import { Input } from '@/components/ui/input';
import { MenuViewport, MENU_TALLEST } from '@/components/ui/menu-viewport';
import { IconCheck, IconSearch } from '@/components/icons';
import { imeKey } from '@/lib/ime';
import { cn } from '@/lib/utils';
export type ChoiceOption = {
  id: string;
  label: string;
  terms?: string;
  aside?: string;
  icon?: ReactNode;
};
export type FieldAction = {
  id: string;
  label: string;
  icon?: ReactNode;
  run: () => void;
  disabled?: boolean;
};
export function ChoiceField({
  label,
  value,
  selected,
  checked,
  keepOpenOnPick = false,
  leadingIcon,
  options,
  onPick,
  actions = [],
  onPendingChange,
  onInput,
  onSubmit,
  multiline = false,
  placeholder,
  disabled = false
}: {
  label: string;
  value: string;
  selected?: string;
  checked?: ReadonlySet<string>;
  keepOpenOnPick?: boolean;
  leadingIcon?: ReactNode;
  options: ChoiceOption[];
  onPick: (id: string) => void;
  actions?: FieldAction[];
  onPendingChange?: (pending: boolean) => void;
  onInput?: (text: string) => void;
  onSubmit?: () => void;
  multiline?: boolean;
  placeholder?: string;
  disabled?: boolean;
}) {
  const uid = useId(),
    list = uid + '-list';
  const [open, setOpen] = useState(false),
    [edited, setEdited] = useState(false),
    [query, setQuery] = useState(''),
    [at, setAt] = useState(-1);
  const chosen = options.find(o => o.id === selected);
  const field = useRef<HTMLInputElement | HTMLTextAreaElement>(null);
  const filtered = options.filter(o => !edited || `${o.label} ${o.terms || ''}`.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()));
  const rows = [...filtered, ...actions];
  const enabled = rows.map((row, index) => 'disabled' in row && row.disabled ? -1 : index).filter(index => index >= 0);
  const icon = edited && !onInput ? <IconSearch /> : leadingIcon ?? chosen?.icon;
  useEffect(() => () => onPendingChange?.(false), [onPendingChange]);
  const reset = () => {
    setEdited(false);
    setQuery('');
    setAt(-1);
    onPendingChange?.(false);
  };
  const take = (id: string) => {
    const act = actions.find(a => a.id === id);
    if (!keepOpenOnPick || act) {
      reset();
      setOpen(false);
    }
    if (act) {
      if (!act.disabled) act.run();
    } else onPick(id);
  };
  useLayoutEffect(() => {
    const el = field.current;
    if (!multiline || !el) return;
    const size = () => {
      el.style.height = 'auto';
      el.style.height = el.scrollHeight + 'px';
    };
    size();
    let width = el.clientWidth;
    const observer = new ResizeObserver(() => {
      if (el.clientWidth !== width) {
        width = el.clientWidth;
        size();
      }
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [value, multiline]);
  const displayed = onInput ? value : edited ? query : value;
  const fieldProps = {
    disabled,
    placeholder,
    'aria-label': label,
    role: 'combobox',
    'aria-expanded': open,
    'aria-controls': list,
    'aria-autocomplete': 'list' as const,
    'aria-activedescendant': open && at >= 0 ? uid + '-' + at : undefined,
    autoComplete: 'off',
    spellCheck: false,
    value: displayed,
    onFocus: (e: FocusEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      reset();
      setOpen(true);
      if (!onInput) e.currentTarget.select();
    },
    onClick: (e: MouseEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      if (!open) {
        reset();
        setOpen(true);
        if (!onInput) e.currentTarget.select();
      }
    },
    onChange: (e: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      setQuery(e.target.value);
      setEdited(true);
      setAt(-1);
      setOpen(true);
      onPendingChange?.(!onInput);
      onInput?.(e.target.value);
    },
    onBlur: () => {
      setOpen(false);
      reset();
    },
    onKeyDown: (e: KeyboardEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      if (imeKey(e)) return;
      if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
        e.preventDefault();
        onSubmit?.();
        return;
      }
      if ((e.key === 'ArrowDown' || e.key === 'ArrowUp') && !(multiline && displayed.includes('\n') && at < 0 && !e.altKey)) {
        e.preventDefault();
        setOpen(true);
        if (enabled.length) setAt(n => {
          const index = enabled.indexOf(n);
          return enabled[e.key === 'ArrowDown' ? (index + 1) % enabled.length : index <= 0 ? enabled.length - 1 : index - 1];
        });
      } else if (open && e.key === 'Enter' && (!onInput || at >= 0)) {
        e.preventDefault();
        const row = rows[at >= 0 ? at : enabled[0]];
        if (row) take(row.id);
      } else if (e.key === 'Escape' && open) {
        e.preventDefault();
        e.stopPropagation();
        setOpen(false);
        reset();
      }
    }
  };
  const renderRow = (o: ChoiceOption | FieldAction, n: number) => (<button key={o.id} id={uid + '-' + n} type="button" role="option" aria-selected={checked ? checked.has(o.id) : o.id === selected} data-choice-option={o.id} data-on={n === at} onMouseDown={e => e.preventDefault()} onClick={() => take(o.id)} disabled={!!('disabled' in o && o.disabled)} className={cn('choice-row disabled:opacity-40 flex min-h-menu-item w-full items-center gap-in rounded-menu-item px-row text-left text-body', n === at && 'glass-sel')}>
     {o.icon && <span className="choice-row-icon" aria-hidden="true">{o.icon}</span>}
     <span className="min-w-0 flex-1 truncate">{o.label}</span>
     {'aside' in o && o.aside && <span className="choice-aside text-caption text-muted-foreground" title={o.aside}>{o.aside}</span>}
     {(checked ? checked.has(o.id) : o.id === selected) && <IconCheck className="size-affordance flex-none" />}
    </button>);
  return <PopoverPrimitive.Root open={open} onOpenChange={setOpen}><div data-choice-field className="choice-field">
  <PopoverPrimitive.Anchor asChild><div className="choice-input">
   {icon && <span className="choice-icon" aria-hidden="true">{icon}</span>}
   {multiline ? <textarea {...fieldProps} ref={el => {
            field.current = el;
          }} rows={1} className={cn('choice-textarea min-h-touch w-full min-w-0 rounded-sm border-0 bg-fill px-in text-body text-foreground placeholder:text-tertiary focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-ring', icon && 'pl-[calc(2*var(--gap-in)+var(--gap-base))]')} /> : <Input {...fieldProps} ref={el => {
            field.current = el;
          }} className={cn(icon && 'pl-[calc(2*var(--gap-in)+var(--gap-base))]')} />}
  </div></PopoverPrimitive.Anchor>
  {open && <PopoverPrimitive.Portal><PopoverPrimitive.Content align="start" sideOffset={4} onOpenAutoFocus={e => e.preventDefault()} onCloseAutoFocus={e => e.preventDefault()} onFocusOutside={e => e.preventDefault()} onPointerDownOutside={e => {
          if (field.current?.contains(e.target as Node)) e.preventDefault();
        }} style={{
          width: "var(--radix-popover-trigger-width)",
          maxHeight: `min(${MENU_TALLEST}px, var(--radix-popover-content-available-height))`
        }} id={list} role="listbox" aria-label={label} aria-multiselectable={checked ? true : undefined} data-menu-frame className="glass z-50 overflow-hidden rounded-menu border-0 bg-glass-menu shadow-[var(--glass-in),var(--glass-shadow)] outline-none">
   <MenuViewport key={filtered.map(r => r.id).join('\0')} active={at} room="var(--radix-popover-content-available-height)">
    {!filtered.length && <div className="px-row py-in text-body text-muted-foreground">{gui.no_matches}</div>}
    {filtered.map(renderRow)}
   </MenuViewport>
   {actions.length > 0 && <div className="flex-none border-t border-border p-menu-inset">{actions.map((action,index)=>renderRow(action,filtered.length+index))}</div>}
  </PopoverPrimitive.Content></PopoverPrimitive.Portal>}
 </div></PopoverPrimitive.Root>;
}
