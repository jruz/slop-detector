defmodule MyApp.MixProject do
  use Mix.Project

  def project do
    [
      app: :my_app,
      version: "0.1.0",
      elixir: "1.15.0",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  defp deps do
    [
      {:phoenix, "1.7.18"},
      {:ecto_sql, "3.13.0"},
      {:postgrex, "0.19.3"},
      {:telemetry_metrics, "1.0.0"},
      {:jason, "1.4.4"}
    ]
  end
end
