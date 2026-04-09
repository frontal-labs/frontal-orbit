const vscode = require("vscode");
const { execFile } = require("child_process");

function activate(context) {
  context.subscriptions.push(
    vscode.commands.registerCommand("orbit.startRepl", () => startRepl()),
    vscode.commands.registerCommand("orbit.askSelection", () => askSelection()),
    vscode.commands.registerCommand("orbit.askInput", () => askInput())
  );
}

function deactivate() {}

function config() {
  return vscode.workspace.getConfiguration("orbit");
}

function orbitCliPath() {
  const value = config().get("cliPath", "orbit");
  return typeof value === "string" && value.trim().length > 0 ? value.trim() : "orbit";
}

function orbitModelArgs() {
  const model = config().get("defaultModel", "");
  if (typeof model === "string" && model.trim().length > 0) {
    return ["--model", model.trim()];
  }
  return [];
}

function startRepl() {
  const terminal = vscode.window.createTerminal("Orbit");
  terminal.show(true);
  terminal.sendText(orbitCliPath(), true);
}

async function askInput() {
  const question = await vscode.window.showInputBox({
    prompt: "Ask Orbit",
    placeHolder: "Explain the active file"
  });
  if (!question || !question.trim()) {
    return;
  }
  await runOrbitPrompt(question.trim());
}

async function askSelection() {
  const editor = vscode.window.activeTextEditor;
  if (!editor) {
    vscode.window.showWarningMessage("Orbit: open an editor first.");
    return;
  }
  const selected = editor.document.getText(editor.selection).trim();
  if (!selected) {
    vscode.window.showWarningMessage("Orbit: select some text first.");
    return;
  }
  const question = await vscode.window.showInputBox({
    prompt: "What should Orbit do with this selection?",
    placeHolder: "Explain this code"
  });
  if (!question || !question.trim()) {
    return;
  }
  const prompt = `${question.trim()}\n\nSelected code:\n${selected}`;
  await runOrbitPrompt(prompt);
}

async function runOrbitPrompt(prompt) {
  const output = vscode.window.createOutputChannel("Orbit");
  output.show(true);
  output.appendLine("Running Orbit...");

  const args = [...orbitModelArgs(), "--output-format", "text", "prompt", prompt];
  const cwd = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath;
  const execOptions = cwd ? { cwd, maxBuffer: 16 * 1024 * 1024 } : { maxBuffer: 16 * 1024 * 1024 };

  const startedAt = Date.now();
  execFile(orbitCliPath(), args, execOptions, (error, stdout, stderr) => {
    const elapsedMs = Date.now() - startedAt;
    output.appendLine(`Orbit finished in ${elapsedMs} ms.`);
    if (stdout && stdout.trim()) {
      output.appendLine("");
      output.appendLine(stdout.trimEnd());
    }
    if (stderr && stderr.trim()) {
      output.appendLine("");
      output.appendLine("stderr:");
      output.appendLine(stderr.trimEnd());
    }

    if (error) {
      const message = `Orbit command failed: ${error.message}`;
      output.appendLine(message);
      vscode.window.showErrorMessage(message);
      return;
    }

    vscode.window.showInformationMessage("Orbit response ready.");
  });
}

module.exports = {
  activate,
  deactivate
};
