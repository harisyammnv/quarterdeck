import { forwardRef } from "react";
import { MagnifyingGlassIcon as MagnifyingGlass, XIcon as X } from "@phosphor-icons/react";
import { Kbd } from "../../ui";

/** The "Find in memory" field: a search input with the / shortcut shown, and a clear button. */
export const FindField = forwardRef<HTMLInputElement, { value: string; onChange: (v: string) => void; placeholder: string; className?: string }>(
  function FindField({ value, onChange, placeholder, className }, ref) {
    return (
      <label className={`qd-cmd memory-find${className ? ` ${className}` : ""}`}>
        <MagnifyingGlass size={16} aria-hidden="true" />
        <span className="sr-only">{placeholder}</span>
        <input
          ref={ref}
          type="search"
          value={value}
          placeholder={placeholder}
          spellCheck={false}
          autoComplete="off"
          maxLength={200}
          onChange={(e) => {
            onChange(e.target.value);
          }}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              if (value) onChange("");
              else e.currentTarget.blur();
              e.stopPropagation();
            }
          }}
        />
        {value ? (
          <button
            type="button"
            className="memory-find-clear"
            aria-label="Clear the search"
            onClick={() => {
              onChange("");
            }}
          >
            <X size={12} weight="bold" />
          </button>
        ) : (
          <Kbd>/</Kbd>
        )}
      </label>
    );
  },
);
