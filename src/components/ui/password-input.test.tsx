import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PasswordInput } from "@/components/ui/password-input";

describe("PasswordInput", () => {
  it("is masked by default and toggles visibility", async () => {
    const user = userEvent.setup();
    render(<PasswordInput aria-label="Пароль" defaultValue="s3cret" />);
    const input = screen.getByLabelText("Пароль");

    expect(input).toHaveAttribute("type", "password");

    await user.click(screen.getByRole("button", { name: "Показать пароль" }));
    expect(input).toHaveAttribute("type", "text");

    await user.click(screen.getByRole("button", { name: "Скрыть пароль" }));
    expect(input).toHaveAttribute("type", "password");
  });

  it("disables autofill and spellcheck", () => {
    render(<PasswordInput aria-label="Пароль" />);
    const input = screen.getByLabelText("Пароль");

    expect(input).toHaveAttribute("autocomplete", "off");
    expect(input).toHaveAttribute("spellcheck", "false");
  });
});
