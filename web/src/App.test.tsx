import { fireEvent, render, screen } from "@testing-library/react";
import { expect, test } from "vitest";

import { App } from "./App";
import { strings } from "./i18n/strings";

function messageToggleAt(index: number) {
  const toggles = screen.getAllByRole("button", {
    name: new RegExp(strings.sampleSender),
  });
  const toggle = toggles.at(index);
  expect(toggle).toBeDefined();
  if (toggle === undefined) {
    throw new Error(`Expected a message toggle at index ${index}`);
  }
  return toggle;
}

test("renders a conversation with the latest message expanded", () => {
  render(<App />);

  expect(
    screen.getByRole("heading", { level: 1, name: strings.sampleSubject }),
  ).toBeTruthy();
  expect(screen.getAllByRole("article")).toHaveLength(3);

  expect(messageToggleAt(0).getAttribute("aria-expanded")).toBe("false");
  expect(messageToggleAt(2).getAttribute("aria-expanded")).toBe("true");
});

test("lets readers expand an earlier message", () => {
  render(<App />);

  const firstMessage = messageToggleAt(0);
  fireEvent.click(firstMessage);

  expect(firstMessage.getAttribute("aria-expanded")).toBe("true");
  expect(screen.getAllByText(strings.reply).length).toBeGreaterThan(1);
});
