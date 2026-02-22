# React Refactoring Patterns

## 1. Extract Custom Hooks
Move logic out of components.

**Before:**
```tsx
const Component = () => {
  const [data, setData] = useState(null);
  useEffect(() => { fetch().then(setData) }, []);
  // ...
}
```

**After:**
```tsx
const Component = () => {
  const { data } = useFetchData(); // Logic hidden
  // ...
}
```

## 2. Composition over Props Drilling
Don't pass props down 4 levels. Use `children` or specialized components.

## 3. Memoization
Refactor expensive calculations with `useMemo` only when Profiler shows lag. DO NOT premature optimize options objects.
