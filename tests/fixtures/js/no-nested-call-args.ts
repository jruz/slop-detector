const renderResult = (value: string): string => value;

const runBash = (command: string): string => command;

export const render = (command: string): string => renderResult(runBash(command));
