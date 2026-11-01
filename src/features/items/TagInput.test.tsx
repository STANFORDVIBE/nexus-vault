import { useState } from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TagInput } from "@/features/items/TagInput";

function Harness({ initial = [] as string[] }) {
  const [tags, setTags] = useState(initial);
  return (
    <>
      <TagInput id="tags" value={tags} onChange={setTags} />
      <output>{tags.join("|")}</output>
    </>
  );
}

describe("TagInput", () => {
  it("adds tags on Enter and comma, skipping duplicates", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    const input = screen.getByRole("textbox");

    await user.type(input, "work{Enter}Home,WORK,");
    expect(screen.getByRole("status")).toHaveTextContent("work|Home");
  });

  it("splits pasted comma-separated text", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByRole("textbox"));
    await user.paste("a, b ,c");
    expect(screen.getByRole("status")).toHaveTextContent("a|b");
    expect(screen.getByRole("textbox")).toHaveValue("c");
  });

  it("removes the last tag with Backspace on an empty input", async () => {
    const user = userEvent.setup();
    render(<Harness initial={["one", "two"]} />);
    await user.type(screen.getByRole("textbox"), "{Backspace}");
    expect(screen.getByRole("status")).toHaveTextContent("one");
  });
});
