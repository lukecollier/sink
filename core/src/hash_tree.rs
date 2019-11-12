use std::hash::Hasher;
use std::iter::FromIterator;

pub trait HashTree {
    fn new(hasher: &mut Hasher) -> Node;
    fn add<'a>(&'a mut self, node: Box<Node>) -> &'a mut Node;
    fn children(&mut self) -> Vec<Box<Node>>;
    fn update(&mut self, hasher: &mut Hasher) -> Self;
    fn hash_key(&mut self, hasher: &mut Hasher);
    fn hash_node(&mut self, hasher: &mut Hasher) -> u64;
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Node {
    key: u64, // any hashing algo output to base 64
    nodes: Vec<Box<Node>>,
    value: Vec<u8> // can be a directory or name
}

impl HashTree for Node {
    fn new(hasher: &mut Hasher) -> Self {
        Self { key: calc_hash(vec![], hasher), nodes: vec![], value: vec![]}
    }

    fn add<'a>(&'a mut self, node: Box<Node>) -> &'a mut Self {
        self.nodes.push(node);
        self
    }

    fn children(&mut self) -> Vec<Box<Node>> {
        let mut buf: Vec<Box<Node>> = Vec::new();
        for node in &self.nodes {
            buf.push(node.clone());
            buf.extend_from_slice(&node.clone().children());
        }
        buf
    }

    fn update(&mut self, mut hasher: &mut Hasher) -> Self  {
        let iterator = self.clone().into_iter();
        iterator.for_each(|mut node| node.hash_key(&mut hasher)); 
        let node: Node = iterator.collect();
        Node::new(&mut hasher)
    }

    fn hash_key(&mut self, hasher: &mut Hasher) {
        self.key = self.hash_node(hasher);
    }

    fn hash_node(&mut self, hasher: &mut Hasher) -> u64 {
		let child_bytes: Vec<u8> = self.children().into_iter()
			.flat_map(|node| node.value).collect();

		let mut total_bytes = Vec::new();
		total_bytes.extend_from_slice(&self.value);
		total_bytes.extend_from_slice(&child_bytes);

		calc_hash(total_bytes, hasher)
    }

}

impl FromIterator<Box<Node>> for Node {
    fn from_iter<I: IntoIterator<Item=Box<Node>>>(iter: I) -> Self {
        // todo put list in root first order, then just take the first using iter.0
        *iter.into_iter().last().expect("couldn't get the last one")
    }
}

impl IntoIterator for Node {
	type Item = Box<Node>;
	type IntoIter = HashTreeIterator;

	fn into_iter(self) -> Self::IntoIter {
		HashTreeIterator::from_root(self)
	}
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct HashTreeIterator {
	queue: Vec<Box<Node>>
}

impl HashTreeIterator {
	fn from_root(root: Node) -> Self {
		let mut queue: Vec<Box<Node>> = vec![Box::new(root.clone())];
		queue.extend_from_slice(&root.nodes);
		for node in &root.nodes {
			queue.extend_from_slice(&node.nodes);
			for inner_node in &node.nodes {
				queue.extend_from_slice(&inner_node.nodes);
			}
		}
		HashTreeIterator { queue: queue }
	}
}

impl Iterator for HashTreeIterator {
	type Item = Box<Node>;
	fn next(&mut self) -> Option<Self::Item> {
		self.queue.pop()
	}
}

fn calc_hash(bytes: Vec<u8>, hasher: &mut Hasher) -> u64 {
    hasher.write(&bytes);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_is_empty() {
        let mut hasher = FakeHash::new();
        let mut expected = Node { key: 1, nodes: vec![], value: vec![]};
        let mut actual = Node::new(&mut hasher);
        assert_eq!(expected, actual);
    }

    #[test]
    fn node_with_four_children_gives_back_four() {
        let mut hasher = FakeHash::new();
        let mut node_one = Node { key: 1, nodes: vec![], value: vec![]};
        let mut node_two = Node { key: 2, nodes: vec![], value: vec![]};
        let mut node_three = Node { key: 3, nodes: vec![], value: vec![]};
        let mut node_four = Node { key: 4, nodes: vec![], value: vec![]};
        let mut node_five = Node { key: 5, nodes: vec![], value: vec![]};

		node_four.add(Box::new(node_five));
		node_three.add(Box::new(node_four));
		node_one.add(Box::new(node_two)).add(Box::new(node_three));
		
		let expected = 4;
		let actual = node_one.children().len();
        assert_eq!(expected, actual);
    }

    #[test]
    fn hash_tree_nodes() {
        let mut hasher = FakeHash::new();
        let mut node_one = Node { key: 1, nodes: vec![], value: vec![]};
        let mut node_two = Node { key: 2, nodes: vec![], value: vec![]};
        let mut node_three = Node { key: 3, nodes: vec![], value: vec![]};
        let mut node_four = Node { key: 4, nodes: vec![], value: vec![]};

		node_three.add(Box::new(node_four));
		node_one.add(Box::new(node_two)).add(Box::new(node_three));
		
		let expected = 1;
		let actual = node_one.hash_node(&mut hasher);
        assert_eq!(expected, actual);
    }

    #[test]
    fn from_iterator() {
        let mut node_one = Node { key: 1, nodes: vec![], value: vec![]};
        let mut node_two = Node { key: 2, nodes: vec![], value: vec![]};
        let mut node_three = Node { key: 3, nodes: vec![], value: vec![]};
        let mut node_four = Node { key: 4, nodes: vec![], value: vec![]};

		node_three.add(Box::new(node_four));
		node_one.add(Box::new(node_two)).add(Box::new(node_three));
		
		let expected_one = node_one.clone();
		let actual_one: Node = node_one.into_iter().collect();
        assert_eq!(expected_one, actual_one);
    }

    #[test]
    fn check_key_correctly_hashes() {
        let mut hasher = FakeHash::new();
        let mut node_one = Node { key: 0, nodes: vec![], value: vec![]};
        let mut node_two = Node { key: 0, nodes: vec![], value: vec![]};
        let mut node_three = Node { key: 0, nodes: vec![], value: vec![]};
        let mut node_four = Node { key: 0, nodes: vec![], value: vec![]};

        node_four.hash_key(&mut hasher);
		node_three.add(Box::new(node_four)).clone();
        node_three.hash_key(&mut hasher);
        node_two.hash_key(&mut hasher);
		node_one.add(Box::new(node_two)).add(Box::new(node_three)).clone();
        node_one.hash_key(&mut hasher);
        
        let mut expected_node_one = Node { key: 4, nodes: vec![], value: vec![]};
        let mut expected_node_two = Node { key: 3, nodes: vec![], value: vec![]};
        let mut expected_node_three = Node { key: 2, nodes: vec![], value: vec![]};
        let mut expected_node_four = Node { key: 1, nodes: vec![], value: vec![]};

		expected_node_three.add(Box::new(expected_node_four)).clone();
		expected_node_one.add(Box::new(expected_node_two)).add(Box::new(expected_node_three)).clone();
		
        assert_eq!(node_one, expected_node_one);
    }

    #[test]
    fn check_update_hashes() {
        let mut hasher = FakeHash::new();
        let mut node_one = Node { key: 0, nodes: vec![], value: vec![]};
        let mut node_two = Node { key: 0, nodes: vec![], value: vec![]};
        let mut node_three = Node { key: 0, nodes: vec![], value: vec![]};
        let mut node_four = Node { key: 0, nodes: vec![], value: vec![]};

		node_three.add(Box::new(node_four)).clone();
		node_one.add(Box::new(node_two)).add(Box::new(node_three)).clone();
        node_one.update(&mut hasher);

        let mut expected_node_one = Node { key: 4, nodes: vec![], value: vec![]};
        let mut expected_node_two = Node { key: 3, nodes: vec![], value: vec![]};
        let mut expected_node_three = Node { key: 2, nodes: vec![], value: vec![]};
        let mut expected_node_four = Node { key: 1, nodes: vec![], value: vec![]};

		expected_node_three.add(Box::new(expected_node_four)).clone();
		expected_node_one.add(Box::new(expected_node_two)).add(Box::new(expected_node_three)).clone();
		
        assert_eq!(node_one, expected_node_one);
    }

	pub struct FakeHash {
		count: u64
	}

	impl FakeHash {
		fn new() -> Self {
			FakeHash { count: 0 }
		}
	}

	impl Hasher for FakeHash {
		fn finish(&self) -> u64 {
			self.count
		}

		fn write(&mut self, _bytes: &[u8]) {
			self.count = self.count + 1;
		}
	}
}
