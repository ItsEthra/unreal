use crate::sdk::{Package, Sdk};
use log::warn;
use petgraph::{
    stable_graph::{NodeIndex, StableGraph},
    Directed,
    Direction::{Incoming, Outgoing},
};

pub(crate) fn eliminate_dependency_cycles(sdk: &mut Sdk) {
    type G = StableGraph<Package, (), Directed>;
    type NI = NodeIndex;

    #[rustfmt::skip]
    fn format_cycle(chain: &[NI], g: &G) -> String {
        use std::fmt::Write;

        let mut s: String = "".into();
        for (i, link) in chain.iter().enumerate() {
            _ = match true {
                _ if i != chain.len() - 1 => write!(s, "{} -> ", g.node_weight(*link).unwrap().ident),
                _ => write!(s, "{}", g.node_weight(*link).unwrap().ident),
            };
        }

        s
    }

    fn inner(current: NI, g: &G, mut chain: Vec<NI>) -> Option<Vec<NI>> {
        let mut out = None;
        for neighbor in g.neighbors(current) {
            if let Some(i) = chain.iter().position(|n| *n == neighbor) {
                chain.push(neighbor);
                return Some(chain.split_off(i));
            }
        }

        for neighbor in g.neighbors(current) {
            let mut copy = chain.clone();
            copy.push(neighbor);
            out = out.or(inner(neighbor, g, copy));
        }

        out
    }

    loop {
        let cycle = |i: NI| inner(i, &sdk.packages, vec![i]);
        let Some(cycle) = sdk.packages.node_indices().find_map(cycle) else {
            break;
        };

        warn!(
            "Eliminating dependency cycle {}",
            format_cycle(&cycle, &sdk.packages)
        );

        let consumer = cycle[0];
        for idx in (cycle[1..cycle.len() - 1]).iter().rev() {
            for dependant in sdk
                .packages
                .neighbors_directed(*idx, Incoming)
                .collect::<Vec<_>>()
            {
                if dependant != consumer {
                    sdk.packages.update_edge(dependant, consumer, ());
                }

                let old = sdk.packages.find_edge(dependant, *idx).unwrap();
                sdk.packages.remove_edge(old);
            }

            for dependency in sdk
                .packages
                .neighbors_directed(*idx, Outgoing)
                .collect::<Vec<_>>()
            {
                if dependency != consumer {
                    sdk.packages.update_edge(consumer, dependency, ());
                }

                let old = sdk.packages.find_edge(*idx, dependency).unwrap();
                sdk.packages.remove_edge(old);
            }

            let Package { objects, .. } = sdk.packages.remove_node(*idx).unwrap();
            for object in &objects {
                sdk.owned.get_mut(&object.fqn()).unwrap().package = cycle[0];
            }

            let consumer = sdk.packages.node_weight_mut(cycle[0]).unwrap();
            consumer.objects.extend(objects);
        }
    }
}
