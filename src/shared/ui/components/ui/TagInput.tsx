import { KeyboardEvent, useState } from 'react';
import { X } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface TagInputProps {
  tags: string[];
  onChange: (tags: string[]) => void;
  placeholder?: string;
  className?: string;
}

export function TagInput({ tags = [], onChange, placeholder, className = '' }: TagInputProps) {
  const { t } = useTranslation(['common']);
  const [inputValue, setInputValue] = useState('');

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === ' ' || e.key === ',' || e.key === 'Enter') {
      e.preventDefault();
      const newTag = inputValue.trim().replace(/,$/, '');
      if (newTag && !tags.includes(newTag)) {
        onChange([...tags, newTag]);
      }
      setInputValue('');
    } else if (e.key === 'Backspace' && inputValue === '' && tags.length > 0) {
      onChange(tags.slice(0, -1));
    }
  };

  const removeTag = (tagToRemove: string) => {
    onChange(tags.filter((t) => t !== tagToRemove));
  };

  return (
    <div
      className={`flex flex-wrap items-center gap-1 rounded-lg border border-base-content/20 bg-base-100 p-1 transition-[border-color] duration-150 focus-within:outline-2 focus-within:outline-primary/50 ${className}`}
    >
      {tags.map((tag) => (
        <span key={tag} className="badge badge-primary gap-1 px-2 py-3 text-xs flex items-center">
          {tag}
          <button
            type="button"
            onClick={() => removeTag(tag)}
            className="workspace-interactive rounded-full p-0.5 hover:bg-primary-focus focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary focus-visible:outline-offset-1"
            aria-label={t('actions.remove', { item: tag })}
          >
            <X size={12} />
          </button>
        </span>
      ))}
      <input
        type="text"
        className="flex-1 min-w-30 bg-transparent outline-none px-2 py-1 text-sm text-base-content"
        placeholder={tags.length === 0 ? placeholder : ''}
        value={inputValue}
        onChange={(e) => setInputValue(e.target.value)}
        onKeyDown={handleKeyDown}
      />
    </div>
  );
}
