import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { it, expect, vi } from "vitest";
import { Menu } from "../src/components";
it("opens with the keyboard, selects, restores focus, and dismisses with Escape", async () => {
  const select = vi.fn();
  const user = userEvent.setup();
  render(
    <Menu
      label="Choose section"
      items={[
        { label: "Library", onSelect: () => {} },
        { label: "Settings", onSelect: select },
      ]}
    >
      Library
    </Menu>,
  );
  const trigger = screen.getByRole("button", { name: "Choose section" });
  trigger.focus();
  await user.keyboard("{ArrowDown}");
  expect(screen.getByRole("menu")).toBeVisible();
  await user.keyboard("{End}{Enter}");
  expect(select).toHaveBeenCalledOnce();
  expect(trigger).toHaveFocus();
  await user.click(trigger);
  await user.keyboard("{Escape}");
  expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  expect(trigger).toHaveFocus();
});

it("dismisses when clicking outside the menu", async () => {
  const user = userEvent.setup();
  render(
    <>
      <Menu label="Actions" items={[{ label: "Play", onSelect: () => {} }]}>
        More
      </Menu>
      <button>Outside</button>
    </>,
  );
  await user.click(screen.getByRole("button", { name: "Actions" }));
  expect(screen.getByRole("menu")).toBeVisible();
  fireEvent.pointerDown(screen.getByText("Outside"), { pointerType: "mouse" });
  expect(screen.queryByRole("menu")).not.toBeInTheDocument();
});
