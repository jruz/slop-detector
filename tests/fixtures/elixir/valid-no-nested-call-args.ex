defmodule NestedCallArgs do
  def render_result(command) do
    output = run_bash(command)
    wrap(output)
  end

  defp wrap(value), do: value

  defp run_bash(command), do: command
end
