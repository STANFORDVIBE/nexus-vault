import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SegmentedControl } from "@/components/ui/segmented-control";

const OPTIONS = [
  { value: "a", label: "Альфа" },
  { value: "b", label: "Бета" },
] as const;

describe("SegmentedControl", () => {
  it("reports the newly selected option", async () => {
    const user = userEvent.setup();
    const onValueChange = vi.fn();
    render(
      <SegmentedControl
        aria-label="Выбор"
        value="a"
        options={OPTIONS}
        onValueChange={onValueChange}
      />,
    );

    await user.click(screen.getByRole("radio", { name: "Бета" }));

    expect(onValueChange).toHaveBeenCalledWith("b");
  });

  it("never deselects the active option", async () => {
    const user = userEvent.setup();
    const onValueChange = vi.fn();
    render(
      <SegmentedControl
        aria-label="Выбор"
        value="a"
        options={OPTIONS}
        onValueChange={onValueChange}
      />,
    );

    await user.click(screen.getByRole("radio", { name: "Альфа" }));

    expect(onValueChange).not.toHaveBeenCalled();
    expect(screen.getByRole("radio", { name: "Альфа" })).toHaveAttribute("aria-checked", "true");
  });
});
