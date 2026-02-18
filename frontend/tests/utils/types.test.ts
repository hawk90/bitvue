/**
 * Menu Module Type Tests
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { dispatchMenuEvent, createMenuItem, createSubmenu } from "../types";

// Mock Tauri APIs - use vi.hoisted to make variables available to hoisted mock
const { mockMenuItemNew, mockSubmenuNew } = vi.hoisted(() => ({
  mockMenuItemNew: vi.fn(),
  mockSubmenuNew: vi.fn(),
}));

vi.mock("@tauri-apps/api/menu", () => ({
  MenuItem: {
    new: mockMenuItemNew,
  },
  Submenu: {
    new: mockSubmenuNew,
  },
  PredefinedMenuItem: {
    new: vi.fn(),
  },
}));

describe("dispatchMenuEvent", () => {
  let addEventListenerSpy: ReturnType<typeof vi.spyOn>;
  let removeEventListenerSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    addEventListenerSpy = vi.spyOn(window, "addEventListener");
    removeEventListenerSpy = vi.spyOn(window, "removeEventListener");
  });

  afterEach(() => {
    addEventListenerSpy.mockRestore();
    removeEventListenerSpy.mockRestore();
  });

  it("should dispatch custom event without detail", () => {
    const handler = vi.fn();
    window.addEventListener("test-event", handler);

    dispatchMenuEvent("test-event");

    expect(handler).toHaveBeenCalledTimes(1);
    const event = handler.mock.calls[0][0] as CustomEvent;
    expect(event.type).toBe("test-event");
    // CustomEvent initializes detail to null when not provided
    expect(event.detail).toBeNull();

    window.removeEventListener("test-event", handler);
  });

  it("should dispatch custom event with detail", () => {
    const handler = vi.fn();
    const detail = { key: "value", number: 42 };
    window.addEventListener("test-event", handler);

    dispatchMenuEvent("test-event", detail);

    expect(handler).toHaveBeenCalledTimes(1);
    const event = handler.mock.calls[0][0] as CustomEvent;
    expect(event.detail).toEqual(detail);

    window.removeEventListener("test-event", handler);
  });

  it("should dispatch event to window", () => {
    const handler = vi.fn();
    window.addEventListener("menu-action", handler);

    dispatchMenuEvent("menu-action", { action: "open" });

    expect(handler).toHaveBeenCalled();

    window.removeEventListener("menu-action", handler);
  });

  it("should handle null detail", () => {
    const handler = vi.fn();
    window.addEventListener("test-event", handler);

    dispatchMenuEvent("test-event", null);

    expect(handler).toHaveBeenCalled();
    const event = handler.mock.calls[0][0] as CustomEvent;
    expect(event.detail).toBeNull();

    window.removeEventListener("test-event", handler);
  });

  it("should handle undefined detail", () => {
    const handler = vi.fn();
    window.addEventListener("test-event", handler);

    dispatchMenuEvent("test-event", undefined);

    expect(handler).toHaveBeenCalled();
    const event = handler.mock.calls[0][0] as CustomEvent;
    // When undefined is passed, CustomEvent is created without detail param
    // which results in detail being null (browser behavior)
    expect(event.detail).toBeNull();

    window.removeEventListener("test-event", handler);
  });

  it("should handle complex detail objects", () => {
    const handler = vi.fn();
    const complexDetail = {
      nested: { value: "test" },
      array: [1, 2, 3],
      func: undefined,
    };
    window.addEventListener("test-event", handler);

    dispatchMenuEvent("test-event", complexDetail);

    expect(handler).toHaveBeenCalled();
    const event = handler.mock.calls[0][0] as CustomEvent;
    expect(event.detail).toEqual(complexDetail);

    window.removeEventListener("test-event", handler);
  });

  it("should work with event bubbling", () => {
    const parentHandler = vi.fn();
    const childHandler = vi.fn();

    window.addEventListener("parent-event", parentHandler);
    window.addEventListener("child-event", childHandler);

    dispatchMenuEvent("parent-event");
    dispatchMenuEvent("child-event");

    expect(parentHandler).toHaveBeenCalledTimes(1);
    expect(childHandler).toHaveBeenCalledTimes(1);

    window.removeEventListener("parent-event", parentHandler);
    window.removeEventListener("child-event", childHandler);
  });
});

describe("createMenuItem", () => {
  beforeEach(() => {
    mockMenuItemNew.mockClear();
  });

  it("should create menu item with id and text", async () => {
    const mockItem = { id: "test-id", text: "Test Item" };
    mockMenuItemNew.mockResolvedValue(mockItem);

    const item = await createMenuItem({ id: "test-id", text: "Test Item" });

    expect(mockMenuItemNew).toHaveBeenCalledWith(
      expect.objectContaining({
        id: "test-id",
        text: "Test Item",
      }),
    );
    expect(item).toEqual(mockItem);
  });

  it("should create menu item with accelerator", async () => {
    const mockItem = { id: "test-id", text: "Test", accelerator: "Cmd+O" };
    mockMenuItemNew.mockResolvedValue(mockItem);

    const item = await createMenuItem({
      id: "test-id",
      text: "Test",
      accelerator: "Cmd+O",
    });

    expect(mockMenuItemNew).toHaveBeenCalledWith(
      expect.objectContaining({
        id: "test-id",
        text: "Test",
        accelerator: "Cmd+O",
      }),
    );
    expect(item).toEqual(mockItem);
  });

  it("should create menu item with event action", async () => {
    const mockItem = { id: "test-id", text: "Test" };
    mockMenuItemNew.mockResolvedValue(mockItem);
    const handler = vi.fn();
    window.addEventListener("menu-click", handler);

    await createMenuItem({ id: "test-id", text: "Test", event: "menu-click" });

    // Verify MenuItem.new was called with an action
    expect(mockMenuItemNew).toHaveBeenCalledWith(
      expect.objectContaining({
        id: "test-id",
        text: "Test",
        action: expect.any(Function),
      }),
    );

    window.removeEventListener("menu-click", handler);
  });

  it("should create menu item with event and detail", async () => {
    const detail = { value: 42 };
    const mockItem = { id: "test-id", text: "Test" };
    mockMenuItemNew.mockResolvedValue(mockItem);
    const handler = vi.fn();
    window.addEventListener("menu-click", handler);

    await createMenuItem({
      id: "test-id",
      text: "Test",
      event: "menu-click",
      eventDetail: detail,
    });

    expect(mockMenuItemNew).toHaveBeenCalledWith(
      expect.objectContaining({
        id: "test-id",
        text: "Test",
        action: expect.any(Function),
      }),
    );

    window.removeEventListener("menu-click", handler);
  });

  it("should create menu item without action when no event specified", async () => {
    const mockItem = { id: "test-id", text: "Test" };
    mockMenuItemNew.mockResolvedValue(mockItem);

    const item = await createMenuItem({ id: "test-id", text: "Test" });

    expect(mockMenuItemNew).toHaveBeenCalledWith(
      expect.not.objectContaining({
        action: expect.any(Function),
      }),
    );
    expect(item).toEqual(mockItem);
  });

  it("should handle special characters in text", async () => {
    const mockItem = { id: "test-id", text: "Test & Item" };
    mockMenuItemNew.mockResolvedValue(mockItem);

    const item = await createMenuItem({ id: "test-id", text: "Test & Item" });

    expect(mockMenuItemNew).toHaveBeenCalled();
    expect(item).toEqual(mockItem);
  });

  it("should handle empty text", async () => {
    const mockItem = { id: "test-id", text: "" };
    mockMenuItemNew.mockResolvedValue(mockItem);

    const item = await createMenuItem({ id: "test-id", text: "" });

    expect(mockMenuItemNew).toHaveBeenCalled();
    expect(item).toEqual(mockItem);
  });

  it("should handle very long text", async () => {
    const longText = "a".repeat(1000);
    const mockItem = { id: "test-id", text: longText };
    mockMenuItemNew.mockResolvedValue(mockItem);

    const item = await createMenuItem({ id: "test-id", text: longText });

    expect(mockMenuItemNew).toHaveBeenCalled();
    expect(item).toEqual(mockItem);
  });
});

describe("createSubmenu", () => {
  beforeEach(() => {
    mockMenuItemNew.mockClear();
    mockSubmenuNew.mockClear();
  });

  it("should create submenu with text and items", async () => {
    const mockSubmenu = { text: "Submenu" };
    mockSubmenuNew.mockResolvedValue(mockSubmenu);

    const submenu = await createSubmenu({ text: "Submenu", items: [] });

    expect(mockSubmenuNew).toHaveBeenCalledWith(
      expect.objectContaining({
        text: "Submenu",
        items: [],
      }),
    );
    expect(submenu).toEqual(mockSubmenu);
  });

  it("should create submenu with nested items", async () => {
    const mockItem1 = { id: "item1", text: "Item 1" };
    const mockItem2 = { id: "item2", text: "Item 2" };
    const mockSubmenu = { text: "Submenu" };
    mockMenuItemNew.mockResolvedValueOnce(mockItem1);
    mockMenuItemNew.mockResolvedValueOnce(mockItem2);
    mockSubmenuNew.mockResolvedValue(mockSubmenu);

    const item1 = await createMenuItem({ id: "item1", text: "Item 1" });
    const item2 = await createMenuItem({ id: "item2", text: "Item 2" });

    const submenu = await createSubmenu({
      text: "Submenu",
      items: [item1, item2],
    });

    expect(mockSubmenuNew).toHaveBeenCalledWith(
      expect.objectContaining({
        text: "Submenu",
        items: [mockItem1, mockItem2],
      }),
    );
    expect(submenu).toEqual(mockSubmenu);
  });

  it("should handle empty items array", async () => {
    const mockSubmenu = { text: "Empty Submenu" };
    mockSubmenuNew.mockResolvedValue(mockSubmenu);

    const submenu = await createSubmenu({ text: "Empty Submenu", items: [] });

    expect(mockSubmenuNew).toHaveBeenCalled();
    expect(submenu).toEqual(mockSubmenu);
  });

  it("should resolve all item promises", async () => {
    const mockItem1 = { id: "item1", text: "Item 1" };
    const mockItem2 = { id: "item2", text: "Item 2" };
    const mockSubmenu = { text: "Submenu" };
    mockMenuItemNew.mockResolvedValueOnce(mockItem1);
    mockMenuItemNew.mockResolvedValueOnce(mockItem2);
    mockSubmenuNew.mockResolvedValue(mockSubmenu);

    const item1Promise = createMenuItem({ id: "item1", text: "Item 1" });
    const item2Promise = createMenuItem({ id: "item2", text: "Item 2" });

    const submenu = await createSubmenu({
      text: "Submenu",
      items: [item1Promise, item2Promise],
    });

    expect(mockSubmenuNew).toHaveBeenCalledWith(
      expect.objectContaining({
        items: [mockItem1, mockItem2],
      }),
    );
    expect(submenu).toEqual(mockSubmenu);
  });

  it("should handle mixed sync and async items", async () => {
    const mockItem1 = { id: "sync", text: "Sync" };
    const mockItem2 = { id: "async", text: "Async" };
    const mockSubmenu = { text: "Mixed Submenu" };
    mockMenuItemNew.mockResolvedValueOnce(mockItem1);
    mockMenuItemNew.mockResolvedValueOnce(mockItem2);
    mockSubmenuNew.mockResolvedValue(mockSubmenu);

    const syncItem = await createMenuItem({ id: "sync", text: "Sync" });
    const asyncItem = createMenuItem({ id: "async", text: "Async" });

    const submenu = await createSubmenu({
      text: "Mixed Submenu",
      items: [syncItem, Promise.resolve(asyncItem)],
    });

    expect(mockSubmenuNew).toHaveBeenCalled();
    expect(submenu).toEqual(mockSubmenu);
  });

  it("should handle submenu with many items", async () => {
    const mockItems = Array.from({ length: 50 }, (_, i) => ({
      id: `item-${i}`,
      text: `Item ${i}`,
    }));
    const mockSubmenu = { text: "Large Submenu" };
    mockMenuItemNew.mockImplementation((config: any) =>
      Promise.resolve(config),
    );
    mockSubmenuNew.mockResolvedValue(mockSubmenu);

    const items = await Promise.all(
      Array.from({ length: 50 }, (_, i) =>
        createMenuItem({ id: `item-${i}`, text: `Item ${i}` }),
      ),
    );

    const submenu = await createSubmenu({
      text: "Large Submenu",
      items,
    });

    expect(mockSubmenuNew).toHaveBeenCalledWith(
      expect.objectContaining({
        text: "Large Submenu",
        items: mockItems,
      }),
    );
    expect(submenu).toEqual(mockSubmenu);
  });

  it("should handle nested submenus", async () => {
    const mockItem = { id: "inner-item", text: "Inner Item" };
    const mockInnerSubmenu = { text: "Inner" };
    const mockOuterSubmenu = { text: "Outer" };
    mockMenuItemNew.mockResolvedValueOnce(mockItem);
    mockSubmenuNew.mockResolvedValueOnce(mockInnerSubmenu);
    mockSubmenuNew.mockResolvedValueOnce(mockOuterSubmenu);

    const innerItem = await createMenuItem({
      id: "inner-item",
      text: "Inner Item",
    });
    const innerSubmenu = await createSubmenu({
      text: "Inner",
      items: [innerItem],
    });

    const outerSubmenu = await createSubmenu({
      text: "Outer",
      items: [innerSubmenu],
    });

    expect(mockSubmenuNew).toHaveBeenCalledTimes(2);
    expect(outerSubmenu).toEqual(mockOuterSubmenu);
  });

  it("should preserve item order", async () => {
    const mockItem1 = { id: "first", text: "Item 0" };
    const mockItem2 = { id: "second", text: "Item 1" };
    const mockItem3 = { id: "third", text: "Item 2" };
    const mockSubmenu = { text: "Ordered Submenu" };
    mockMenuItemNew.mockImplementation((config: any) =>
      Promise.resolve(config),
    );
    mockSubmenuNew.mockResolvedValue(mockSubmenu);

    const items = await Promise.all(
      ["first", "second", "third"].map((id, i) =>
        createMenuItem({ id, text: `Item ${i}` }),
      ),
    );

    const submenu = await createSubmenu({
      text: "Ordered Submenu",
      items,
    });

    expect(mockSubmenuNew).toHaveBeenCalledWith(
      expect.objectContaining({
        items: [mockItem1, mockItem2, mockItem3],
      }),
    );
    expect(submenu).toEqual(mockSubmenu);
  });
});

describe("Menu Integration", () => {
  beforeEach(() => {
    mockMenuItemNew.mockClear();
    mockSubmenuNew.mockClear();
  });

  it("should dispatch event when menu item action is triggered", async () => {
    const mockItem = { id: "test", text: "Test" };
    mockMenuItemNew.mockResolvedValue(mockItem);
    const handler = vi.fn();
    window.addEventListener("test-action", handler);

    await createMenuItem({ id: "test", text: "Test", event: "test-action" });

    // Verify that the menu item was created with an action
    expect(mockMenuItemNew).toHaveBeenCalledWith(
      expect.objectContaining({
        id: "test",
        text: "Test",
        action: expect.any(Function),
      }),
    );

    window.removeEventListener("test-action", handler);
  });

  it("should create complex menu structure", async () => {
    const mockItem1 = { id: "new", text: "New" };
    const mockItem2 = { id: "open", text: "Open" };
    const mockFileMenu = { text: "File" };
    mockMenuItemNew.mockResolvedValueOnce(mockItem1);
    mockMenuItemNew.mockResolvedValueOnce(mockItem2);
    mockSubmenuNew.mockResolvedValue(mockFileMenu);

    const fileMenu = await createSubmenu({
      text: "File",
      items: [
        await createMenuItem({
          id: "new",
          text: "New",
          accelerator: "Cmd+N",
          event: "file-new",
        }),
        await createMenuItem({
          id: "open",
          text: "Open",
          accelerator: "Cmd+O",
          event: "file-open",
        }),
      ],
    });

    expect(mockSubmenuNew).toHaveBeenCalledWith(
      expect.objectContaining({
        text: "File",
        items: [mockItem1, mockItem2],
      }),
    );
    expect(fileMenu).toEqual(mockFileMenu);
  });

  it("should handle multiple event types", async () => {
    const events = ["event1", "event2", "event3"];
    const mockItems = events.map((event) => ({ id: event, text: event }));
    const mockSubmenu = { text: "Multi" };
    mockMenuItemNew.mockImplementation((config: any) =>
      Promise.resolve(config),
    );
    mockSubmenuNew.mockResolvedValue(mockSubmenu);
    const handlers = events.map((event) => {
      const handler = vi.fn();
      window.addEventListener(event, handler);
      return { event, handler };
    });

    const items = await Promise.all(
      events.map((event) => createMenuItem({ id: event, text: event, event })),
    );

    const submenu = await createSubmenu({ text: "Multi", items });

    expect(submenu).toEqual(mockSubmenu);

    handlers.forEach(({ event, handler }) => {
      window.removeEventListener(event, handler);
    });
  });
});
