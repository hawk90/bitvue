/**
 * Context Factory - Factory pattern for creating React contexts
 *
 * This module provides a factory pattern for creating React contexts with
 * consistent error handling, default values, and provider components.
 */

import React, {
  createContext,
  useContext,
  ReactNode,
  Context,
  useCallback,
} from "react";

// =============================================================================
// Context Configuration Types
// =============================================================================

/**
 * Error handler for context access
 */
export type ContextErrorHandler<T> = (
  contextName: string,
  value: T | null,
) => Error;

/**
 * Default error handler that throws an error
 */
export const defaultErrorHandler: ContextErrorHandler<unknown> = (
  contextName,
  value,
) => {
  return new Error(
    `${contextName} context value ${value === null ? "is null" : "is undefined"}. ` +
      `Did you forget to wrap your component in a ${contextName}Provider?`,
  );
};

/**
 * Context configuration options
 */
export interface ContextConfig<T> {
  /** Name of the context (for error messages) */
  name: string;
  /** Default value for the context */
  defaultValue: T;
  /** Whether to use strict mode (throw error if accessed outside provider) */
  strict?: boolean;
  /** Custom error handler */
  errorHandler?: ContextErrorHandler<T>;
  /** Zod schema for validation (optional) */
  schema?: { parse: (value: unknown) => T };
}

/**
 * Created context with provider and hook
 */
export interface CreatedContext<T> {
  /** The React context */
  Context: Context<T>;
  /** Provider component */
  Provider: React.FC<{ children: ReactNode; value?: T }>;
  /** Hook to access the context */
  useHook: () => T;
  /** Hook to access the context with optional default */
  useHookOrDefault: (defaultValue: T) => T;
  /** Context name */
  name: string;
}

// =============================================================================
// Context Factory
// =============================================================================

/**
 * Create a React context with provider and hook
 *
 * @example
 * ```tsx
 * import { createContextFactory } from './ContextFactory';
 *
 * interface Theme {
 *   primary: string;
 *   secondary: string;
 * }
 *
 * const { Context: ThemeContext, Provider: ThemeProvider, useTheme } = createContextFactory<Theme>({
 *   name: 'Theme',
 *   defaultValue: { primary: '#007bff', secondary: '#6c757d' },
 *   strict: true,
 * });
 * ```
 */
export function createContextFactory<T>(
  config: ContextConfig<T>,
): CreatedContext<T> {
  const {
    name,
    defaultValue,
    strict = false,
    errorHandler = defaultErrorHandler,
    schema,
  } = config;

  // Create the React context
  const Context = createContext<T>(defaultValue);
  Context.displayName = `${name}Context`;

  // Create provider component
  const Provider: React.FC<{ children: ReactNode; value?: T }> = ({
    children,
    value: providedValue,
  }) => {
    const finalValue = providedValue ?? defaultValue;

    // Validate with schema if provided
    if (schema) {
      try {
        schema.parse(finalValue);
      } catch (error) {
        console.error(`${name} context validation failed:`, error);
        throw new Error(`${name} context validation failed`);
      }
    }

    return <Context.Provider value={finalValue}>{children}</Context.Provider>;
  };

  Provider.displayName = `${name}Provider`;

  // Create hook to access context
  const useHook = (): T => {
    const contextValue = useContext(Context);

    // In strict mode, throw error if context value equals default and wasn't provided
    if (strict) {
      // Check if we're inside a provider by comparing to default
      // This is a simple heuristic - for production, you might want a different approach
      const isUsingDefault =
        JSON.stringify(contextValue) === JSON.stringify(defaultValue);

      if (isUsingDefault) {
        const error = errorHandler(name, contextValue);
        throw error;
      }
    }

    return contextValue;
  };

  useHook.displayName = `use${name}`;

  // Create hook with default value fallback
  const useHookOrDefault = useCallback((fallbackValue: T): T => {
    const contextValue = useContext(Context);
    return contextValue ?? fallbackValue;
  }, []);

  useHookOrDefault.displayName = `use${name}OrDefault`;

  return {
    Context,
    Provider,
    useHook,
    useHookOrDefault,
    name,
  };
}

// =============================================================================
// Specialized Context Factories
// =============================================================================

/**
 * Create a context with setter hook
 *
 * @example
 * ```tsx
 * const { Provider: CounterProvider, useCounter, useSetCounter } = createSetterContext({
 *   name: 'Counter',
 *   defaultValue: 0,
 * });
 * ```
 */
export interface SetterContext<T> extends CreatedContext<T> {
  /** Hook to get the setter function */
  useSetter: () => React.Dispatch<React.SetStateAction<T>>;
}

export function createSetterContext<T>(
  config: ContextConfig<T>,
): SetterContext<T> {
  const {
    Context,
    Provider: BaseProvider,
    useHook,
    useHookOrDefault,
    name,
  } = createContextFactory(config);

  // Create a provider that also provides the setter
  const Provider: React.FC<{ children: ReactNode; value?: T }> = ({
    children,
    value: providedValue,
  }) => {
    // Store state in provider
    const [state, setState] = React.useState<T>(
      providedValue ?? config.defaultValue,
    );

    return (
      <Context.Provider value={state}>
        {/* Create a separate context for the setter */}
        {React.cloneElement(
          children as React.ReactElement,
          { [`${name.toLowerCase()}Setter`]: setState } as any,
        )}
      </Context.Provider>
    );
  };

  // Note: This is a simplified version - in production you'd want
  // a more sophisticated approach with separate contexts for value and setter
  const useSetter = (): React.Dispatch<React.SetStateAction<T>> => {
    // For a complete implementation, you'd need a separate setter context
    throw new Error(
      "useSetter requires a separate setter context implementation",
    );
  };

  return {
    Context,
    Provider,
    useHook,
    useHookOrDefault,
    useSetter,
    name,
  };
}

/**
 * Create a reducer-based context
 *
 * @example
 * ```tsx
 * type CounterAction = { type: 'increment' } | { type: 'decrement' };
 *
 * const { Provider: CounterProvider, useCounter, useCounterDispatch } =
 *   createReducerContext({
 *     name: 'Counter',
 *     initialValue: 0,
 *     reducer: (state, action) => {
 *       switch (action.type) {
 *         case 'increment': return state + 1;
 *         case 'decrement': return state - 1;
 *         default: return state;
 *       }
 *     },
 *   });
 * ```
 */
export interface ReducerContext<T, A> extends CreatedContext<T> {
  /** Hook to get the dispatch function */
  useDispatch: () => React.Dispatch<A>;
}

export function createReducerContext<T, A>(config: {
  name: string;
  initialValue: T;
  reducer: (state: T, action: A) => T;
}): ReducerContext<T, A> {
  const { name, initialValue, reducer } = config;

  // Create context for state
  const StateContext = createContext<T>(initialValue);
  StateContext.displayName = `${name}StateContext`;

  // Create context for dispatch
  const DispatchContext = createContext<React.Dispatch<A>>(() => initialValue);
  DispatchContext.displayName = `${name}DispatchContext`;

  const Provider: React.FC<{ children: ReactNode }> = ({ children }) => {
    const [state, dispatch] = React.useReducer(reducer, initialValue);

    return (
      <StateContext.Provider value={state}>
        <DispatchContext.Provider value={dispatch}>
          {children}
        </DispatchContext.Provider>
      </StateContext.Provider>
    );
  };

  Provider.displayName = `${name}Provider`;

  const useHook = (): T => {
    const state = useContext(StateContext);
    if (state === null) {
      throw new Error(`${name}State context is null`);
    }
    return state;
  };

  const useHookOrDefault = useCallback((fallbackValue: T): T => {
    const state = useContext(StateContext);
    return state ?? fallbackValue;
  }, []);

  const useDispatch = (): React.Dispatch<A> => {
    const dispatch = useContext(DispatchContext);
    if (!dispatch) {
      throw new Error(`${name}Dispatch context is null`);
    }
    return dispatch;
  };

  return {
    Context: StateContext,
    Provider,
    useHook,
    useHookOrDefault,
    useDispatch,
    name,
  };
}

// =============================================================================
// Pre-configured Context Factories
// =============================================================================

/**
 * Create a context for async data (loading state)
 *
 * @example
 * ```tsx
 * interface UserData {
 *   id: string;
 *   name: string;
 * }
 *
 * const { Provider: UserDataProvider, useUserData, useUserLoading } = createAsyncContext<UserData>({
 *   name: 'UserData',
 *   initialValue: null,
 * });
 * ```
 */
export interface AsyncContext<T> extends CreatedContext<T | null> {
  /** Hook to get loading state */
  useLoading: () => boolean;
  /** Hook to get error */
  useError: () => Error | null;
  /** Hook to trigger refetch */
  useRefetch: () => () => void;
}

export function createAsyncContext<T>(config: {
  name: string;
  initialValue: T | null;
}): AsyncContext<T> {
  const {
    Context,
    Provider: BaseProvider,
    useHook: useBaseHook,
    useHookOrDefault,
    name,
  } = createContextFactory<T | null>({
    name: config.name,
    defaultValue: config.initialValue,
    strict: false,
  });

  interface AsyncState {
    data: T | null;
    loading: boolean;
    error: Error | null;
  }

  const AsyncContext = createContext<AsyncState>({
    data: config.initialValue,
    loading: false,
    error: null,
  });

  const Provider: React.FC<{
    children: ReactNode;
    value?: T | null;
    loading?: boolean;
    error?: Error | null;
  }> = ({ children, value, loading = false, error = null }) => {
    return (
      <AsyncContext.Provider value={{ data: value ?? null, loading, error }}>
        <BaseProvider value={value ?? null}>{children}</BaseProvider>
      </AsyncContext.Provider>
    );
  };

  Provider.displayName = `${name}AsyncProvider`;

  const useHook = (): T | null => {
    return useBaseHook();
  };

  const useLoading = (): boolean => {
    const { loading } = useContext(AsyncContext);
    return loading;
  };

  const useError = (): Error | null => {
    const { error } = useContext(AsyncContext);
    return error;
  };

  const useRefetch = () => {
    // This would need to be implemented with a fetch function
    return () => {
      // No-op for now
    };
  };

  return {
    Context,
    Provider,
    useHook,
    useHookOrDefault,
    useLoading,
    useError,
    useRefetch,
    name,
  };
}

// =============================================================================
// Context Utilities
// =============================================================================

/**
 * Merge multiple contexts into a single provider
 *
 * @example
 * ```tsx
 * const { Provider: UserDataProvider, useUser } = createContextFactory({ ... });
 * const { Provider: ThemeProvider, useTheme } = createContextFactory({ ... });
 *
 * const AppProviders = mergeContexts([
 *   [UserDataContext, { userData: defaultUserData }],
 *   [ThemeContext, { theme: defaultTheme }],
 * ]);
 *
 * function App() {
 *   return (
 *     <AppProviders>
 *       <YourApp />
 *     </AppProviders>
 *   );
 * }
 * ```
 */
export function mergeContexts(
  contexts: Array<[Context<unknown>, { value?: unknown }]>,
): React.FC<{ children: ReactNode }> {
  const MergedProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
    return (
      <>
        {contexts.reduceRight((acc, [Context, config], index) => {
          const Provider = Context.Provider as any;
          return (
            <Provider value={config.value} key={index}>
              {acc}
            </Provider>
          );
        }, children as ReactNode)}
      </>
    );
  };

  return MergedProvider;
}

/**
 * HOC to inject context values into component props
 *
 * @example
 * ```tsx
 * interface MyComponentProps {
 *   counter: number;
 * }
 *
 * const withCounter = withContext(CounterContext, 'counter');
 *
 * const MyComponent = withCounter(({ counter }) => {
 *   return <div>Count: {counter}</div>;
 * });
 * ```
 */
// DISABLED: TypeScript Record type causing runtime compilation issue
// export function withContext<T, P extends Record<string, T>>(
//   context: Context<T>,
//   propName: keyof P
// ): <PWithoutContext extends Omit<P, keyof P>>(
//   Component: React.FC<PWithoutContext & P>
// ) => React.FC<PWithoutContext> {
//   return function WithContextComponent(Component: React.FC<any>) {
//     const WrappedComponent: React.FC<any> = (props) => {
//       const contextValue = useContext(context);
//       return <Component {...props} {...{ [propName]: contextValue }} />;
//     };
//
//     WrappedComponent.displayName = `withContext(${Component.displayName || Component.name})`;
//
//     return WrappedComponent;
//   };
// }

// =============================================================================
// Pre-built Context Instances (Example)
// =============================================================================

/**
 * Example: Create a Theme context using the factory
 */
export const exampleThemeContext = createContextFactory<{
  primary: string;
  secondary: string;
  mode: "light" | "dark";
}>({
  name: "Theme",
  defaultValue: {
    primary: "#007bff",
    secondary: "#6c757d",
    mode: "light",
  },
  strict: false,
});

/**
 * Example: Create a Layout context using the factory
 */
export const exampleLayoutContext = createContextFactory<{
  sidebarOpen: boolean;
  panelSizes: number[];
}>({
  name: "Layout",
  defaultValue: {
    sidebarOpen: true,
    panelSizes: [300, 400, 300],
  },
  strict: false,
});
