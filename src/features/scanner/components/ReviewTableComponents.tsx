import { useRef, useEffect, useState, HTMLProps } from 'react';

export function IndeterminateCheckbox({
  isIndeterminate,
  className = '',
  ...rest
}: { isIndeterminate?: boolean } & HTMLProps<HTMLInputElement>) {
  const ref = useRef<HTMLInputElement>(null!);

  useEffect(() => {
    if (typeof isIndeterminate === 'boolean') {
      ref.current.indeterminate = !rest.checked && isIndeterminate;
    }
  }, [ref, isIndeterminate, rest.checked]);

  return <input type="checkbox" ref={ref} className={className + ' cursor-pointer'} {...rest} />;
}

export function EditableCell({
  value: initialValue,
  rawName,
  path,
  onRename,
}: {
  value: string;
  rawName: string;
  path: string;
  onRename: (path: string, newName: string) => void;
}) {
  const [isEditing, setIsEditing] = useState(false);
  const [value, setValue] = useState(initialValue);

  // Sync internal state if prop changes (external update)
  useEffect(() => {
    setValue(initialValue);
  }, [initialValue]);

  const onBlur = () => {
    setIsEditing(false);
    if (value !== initialValue) {
      onRename(path, value);
    }
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      onBlur();
    } else if (e.key === 'Escape') {
      setValue(initialValue);
      setIsEditing(false);
    }
  };

  if (isEditing) {
    return (
      <input
        autoFocus
        className="input input-xs input-bordered w-full"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onBlur={onBlur}
        onKeyDown={onKeyDown}
      />
    );
  }

  return (
    <div
      className="flex flex-col cursor-text hover:bg-base-200 p-1 rounded"
      onDoubleClick={() => setIsEditing(true)}
      title="Double-click to rename"
    >
      <span className="font-bold text-sm">{value}</span>
      <span className="text-[10px] text-base-content/40 font-mono truncate max-w-[200px]">
        {rawName}
      </span>
    </div>
  );
}
