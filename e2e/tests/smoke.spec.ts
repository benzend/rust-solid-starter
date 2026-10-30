import { expect, type Page, test } from "@playwright/test";

// A smoke pass over the whole app: the checks a reviewer would click through on
// a preview before trusting a pull request. Each test saves a full-page
// screenshot to results/screens/ so the PR comment can point at what it saw.

test.describe.configure({ mode: "serial" });

const email = `e2e-${Date.now()}@example.com`;
const password = "correct-horse-battery";

// Uncaught errors and 5xx responses fail the test they happen in, even when the
// page still looks fine. 4xx responses are expected (logged-out /api/me, bad
// input) and the browser's own "Failed to load resource" lines for them are skipped.
const pageProblems = new WeakMap<Page, string[]>();

test.beforeEach(({ page }) => {
  const problems: string[] = [];
  page.on("pageerror", (error) => problems.push(`Uncaught: ${error.message}`));
  page.on("console", (msg) => {
    if (msg.type() === "error" && !msg.text().startsWith("Failed to load resource")) {
      problems.push(`Console: ${msg.text()}`);
    }
  });
  page.on("response", (response) => {
    if (response.status() >= 500) {
      const id = response.headers()["x-request-id"];
      problems.push(`${response.status()} ${response.url()}${id ? ` (request ${id})` : ""}`);
    }
  });
  pageProblems.set(page, problems);
});

test.afterEach(async ({ page }, testInfo) => {
  // API-only tests never open a page, so there's nothing to capture.
  if (page.url() !== "about:blank") {
    const name = testInfo.title.replace(/\W+/g, "-").toLowerCase();
    await page.screenshot({ path: `results/screens/${name}.png`, fullPage: true });
  }
  expect(pageProblems.get(page), "errors in the browser").toEqual([]);
});

test("health check passes, database included", async ({ request }) => {
  const response = await request.get("/api/health");
  expect(response.status()).toBe(200);
});

test("signing up lands on an empty todo list", async ({ page }) => {
  await page.goto("/signup");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(password);
  await page.getByRole("button", { name: "Sign up" }).click();

  await expect(page).toHaveURL("/");
  await expect(page.getByText(email)).toBeVisible();
  await expect(page.getByText("Nothing yet.")).toBeVisible();
});

test("todos can be added, completed and deleted", async ({ page }) => {
  await logIn(page);

  await page.getByLabel("New todo").fill("Check the preview");
  await page.getByRole("button", { name: "Add" }).click();
  await expect(page.getByText("Check the preview")).toBeVisible();
  await expect(page.getByText("1 left")).toBeVisible();

  await page.getByLabel('Mark "Check the preview" done').check();
  await expect(page.getByText("0 left")).toBeVisible();

  // Reloading proves the change reached Postgres, not just the page's state.
  await page.reload();
  await expect(page.getByLabel('Mark "Check the preview" done')).toBeChecked();

  await page.getByRole("button", { name: 'Delete "Check the preview"' }).click();
  await expect(page.getByText("Nothing yet.")).toBeVisible();
});

test("logging out and back in keeps the account", async ({ page }) => {
  await logIn(page);
  await page.getByRole("button", { name: "Log out" }).click();
  await expect(page).toHaveURL("/login");

  // Logged out, the todo list sends you to the login page.
  await page.goto("/");
  await expect(page).toHaveURL(/\/login/);

  await logIn(page);
  await expect(page.getByText(email)).toBeVisible();
});

test("about and unknown pages render", async ({ page }) => {
  await page.goto("/about");
  await expect(page.getByText("About this starter")).toBeVisible();

  await page.goto("/no-such-page");
  await expect(page.getByRole("heading", { name: "Page not found" })).toBeVisible();
});

async function logIn(page: Page) {
  await page.goto("/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(password);
  await page.getByRole("button", { name: "Log in" }).click();
  await expect(page).toHaveURL("/");
}
