pub mod collect;
pub mod operations;

use collect::{
    common::{DumpConfig, OutputFormat},
    error::Error,
    rest_wrapper,
    utils::log,
};
use operations::Resource;
use std::collections::HashMap;

use crate::collect::resource_dump::ResourceDumper;
use plugin::ExecuteDumpOperation;
use std::path::PathBuf;

/// Collects state & log information of mayastor services running in the system and dump them.
#[derive(Debug, Clone, clap::Args)]
pub struct SupportArgs {
    /// Specifies the timeout value to interact with other modules of system
    #[clap(global = true, long, short, default_value = "10s")]
    timeout: humantime::Duration,

    /// Period states to collect all logs from last specified duration
    #[clap(global = true, long, short, default_value = "24h")]
    since: humantime::Duration,

    /// Endpoint of LOKI service, if left empty then it will try to parse endpoint
    /// from Loki service(K8s service resource), if the tool is unable to parse
    /// from service then logs will be collected using Kube-apiserver
    #[clap(global = true, short, long)]
    loki_endpoint: Option<String>,

    /// Endpoint of ETCD service, if left empty then will be parsed from the internal service name
    #[clap(global = true, short, long)]
    etcd_endpoint: Option<String>,

    /// Output directory path to store archive file
    #[clap(global = true, long, short = 'd', default_value = "./")]
    output_directory_path: String,

    /// Kubernetes namespace of mayastor service
    #[clap(global = true, long, short = 'n', default_value = "mayastor")]
    namespace: String,

    /// Path to kubeconfig file.
    #[clap(skip)]
    pub kubeconfig: Option<PathBuf>,
}

/// Supportability - collects state & log information of services and dumps it to a tar file.
#[derive(Debug, Clone, clap::Args)]
#[clap(
    after_help = "Supportability - collects state & log information of services and dumps it to a tar file."
)]
pub struct DumpArgs {
    #[clap(flatten)]
    pub args: SupportArgs,
    #[clap(subcommand)]
    resource: Resource,
}

#[async_trait::async_trait(?Send)]
impl ExecuteDumpOperation for DumpArgs {
    type Args = ();
    type Error = anyhow::Error;
    async fn execute(
        &self,
        _cli_args: &Self::Args,
        host_name_required_svcs: HashMap<&'static str, bool>,
    ) -> Result<(), Self::Error> {
        self.resource
            .execute(&self.args, host_name_required_svcs)
            .await
    }
}

#[async_trait::async_trait(?Send)]
impl ExecuteDumpOperation for Resource {
    type Args = SupportArgs;
    type Error = anyhow::Error;

    async fn execute(
        &self,
        cli_args: &Self::Args,
        host_name_required_svcs: HashMap<&'static str, bool>,
    ) -> Result<(), Self::Error> {
        // let host_name_required_svcs: HashMap<&'static str, bool> =
        //     HashMap::from([
        //         (MAYASTOR_SERVICE, true),
        //         (ETCD_SERVICE, true),
        //         (CSI_NODE_SERVICE, true),
        //         (AGENT_HA_NODE_SERVICE, true),
        //         (NATS_SERVICE, true),
        //     ]);

        let config = kube_proxy::ConfigBuilder::default_api_rest()
            .with_kube_config(cli_args.kubeconfig.clone())
            .with_timeout(*cli_args.timeout)
            .with_target_mod(|t| t.with_namespace(&cli_args.namespace))
            .build()
            .await?;

        let rest_client = rest_wrapper::RestClient::new_with_config(config);

        execute_resource_dump(
            cli_args.clone(),
            rest_client,
            cli_args.kubeconfig.clone(),
            self.clone(),
            &host_name_required_svcs,
        )
        .await
        .map_err(|e| anyhow::anyhow!("{:?}", e))
    }
}

async fn execute_resource_dump(
    cli_args: SupportArgs,
    rest_client: rest_wrapper::RestClient,
    kube_config_path: Option<PathBuf>,
    resource: Resource,
    host_name_required_svcs: &HashMap<&'static str, bool>,
) -> Result<(), Error> {
    let mut config = DumpConfig {
        rest_client: rest_client.clone(),
        output_directory: cli_args.output_directory_path,
        namespace: cli_args.namespace,
        loki_uri: cli_args.loki_endpoint,
        etcd_uri: cli_args.etcd_endpoint,
        since: cli_args.since,
        kube_config_path,
        timeout: cli_args.timeout,
        output_format: OutputFormat::Tar,
    };
    let mut errors = Vec::new();
    match resource {
        Resource::Loki => {
            let mut system_dumper =
                collect::system_dump::SystemDumper::get_or_panic_system_dumper(config, true).await;
            log("Completed collection of topology information".to_string());

            system_dumper
                .collect_and_dump_loki_logs(host_name_required_svcs)
                .await?;
            if let Err(e) = system_dumper.fill_archive_and_delete_tmp() {
                log(format!("Failed to copy content to archive, error: {e:?}"));
                errors.push(e);
            }
        }
        Resource::System(args) => {
            let mut system_dumper = collect::system_dump::SystemDumper::get_or_panic_system_dumper(
                config,
                args.disable_log_collection,
            )
            .await;
            if let Err(e) = system_dumper.dump_system(host_name_required_svcs).await {
                // NOTE: We also need to log error content into Supportability log file
                log(format!("Failed to dump system state, error: {e:?}"));
                errors.push(e);
            }
            if let Err(e) = system_dumper.fill_archive_and_delete_tmp() {
                log(format!("Failed to copy content to archive, error: {e:?}"));
                errors.push(e);
            }
        }
        Resource::Etcd { stdout } => {
            config.output_format = if stdout {
                OutputFormat::Stdout
            } else {
                OutputFormat::Tar
            };
            let mut dumper = ResourceDumper::get_or_panic_resource_dumper(config).await;
            if let Err(e) = dumper.dump_etcd().await {
                log(format!("Failed to dump etcd information, Error: {e:?}"));
                errors.push(e);
            }
        }
    }
    if !errors.is_empty() {
        return Err(Error::MultipleErrors(errors));
    }
    println!("Completed collection of dump !!");
    Ok(())
}
